import os
import platform
import random
from collections.abc import Mapping
from datetime import datetime
from threading import Event, Lock, Thread
from time import time

import win32api
import win32con
from playsound3 import playsound

from core.events import mediator
from core.execution_control import check_cancelled, interruptible_sleep, wait_for_event
from core.i18n import noop
from module.after_completion_types import ACTION_EXIT_AALC, ACTION_EXIT_EMULATOR, ACTION_EXIT_GAME
from module.automation import auto
from module.config import TeamSetting, cfg
from module.decorator.decorator import begin_and_finish_time_log
from module.game_and_screen import game_process, screen
from module.game_and_screen.hdr import get_monitor_hdr_info
from module.logger import log
from module.my_error.my_error import (
    backMainWinError,
    cannotOperateGameError,
    netWorkUnstableError,
    notWaitError,
    unableToFindTeamError,
    unexpectNumError,
    userStopError,
    withOutAdminError,
    withOutGameWinError,
    withOutPicError,
)
from module.notification.toast import TemplateToast, send_toast
from module.system_actions import (
    apply_power_keep_awake,
    execute_after_completion,
    get_after_completion_config,
)
from tasks.base.back_init_menu import back_init_menu
from tasks.base.make_enkephalin_module import (
    lunacy_to_enkephalin,
    make_enkephalin_module,
)
from tasks.base.retry_monitor import retry_monitor
from tasks.battle import battle
from tasks.daily.get_prize import get_mail_prize, get_pass_prize
from tasks.daily.luxcavation import EXP_luxcavation, thread_luxcavation
from tasks.mirror.mirror import Mirror
from tasks.teams.team_formation import select_battle_team
from utils.path_manager import path_manager
from utils.utils import calculate_the_teams, check_hard_mirror_time, get_day_of_week

_RUNNER_TRUE_VALUES = frozenset({"1", "true", "yes", "on"})


def _runner_mode_enabled() -> bool:
    """Return whether this task is running inside the one-shot Runner.

    ``AALC_RUNNER_MODE`` is the task-facing name.  The bootstrap historically
    exported ``AALC_EXECUTION_RUNNER``; accepting both keeps old packaged
    launchers compatible during the migration.
    """

    value = os.environ.get("AALC_RUNNER_MODE", os.environ.get("AALC_EXECUTION_RUNNER", ""))
    return str(value).strip().lower() in _RUNNER_TRUE_VALUES


def _serialise_after_completion_request(
    actions: list[str] | tuple[str, ...],
    power_action: str,
    *,
    run_id: str | None = None,
    outcome: str = "completed",
    forced: bool = False,
) -> dict[str, object]:
    """Build the small JSON-safe handoff consumed by RunnerTaskHost."""

    normalised_actions = [str(getattr(action, "value", action)) for action in actions]
    runner_actions = [action for action in normalised_actions if action in {ACTION_EXIT_GAME.value, ACTION_EXIT_EMULATOR.value}]
    sidecar_actions = [action for action in normalised_actions if action == ACTION_EXIT_AALC.value]
    request: dict[str, object] = {
        "actions": normalised_actions,
        "runnerActions": runner_actions,
        "sidecarActions": sidecar_actions,
        "powerAction": str(getattr(power_action, "value", power_action)),
        "runId": run_id,
        "outcome": outcome,
        "forced": bool(forced),
        "requiresDeviceLease": bool(runner_actions),
        "deviceDisposition": "restore",
    }
    return request


def _runner_context_has_lease(context: object | None) -> bool:
    if context is None:
        return False
    for name in ("device_lease_valid", "lease_valid", "has_device_lease"):
        value = getattr(context, name, None)
        if value is not None:
            return bool(value() if callable(value) else value)
    spec = getattr(context, "spec", None)
    target = getattr(spec, "device_target", None)
    if target is None and isinstance(spec, Mapping):
        target = spec.get("deviceTarget", spec.get("device_target"))
    return bool(getattr(context, "runner_owned_controller", None) is not None and target)


def _execute_runner_device_actions(request: dict[str, object], context: object | None) -> None:
    """Execute only successful-run device actions when a controller is injected.

    The normal bootstrap currently leaves controller creation to a later
    integration step, so absent a controller this function intentionally keeps
    the request pending for RunnerHost rather than touching a global device.
    """

    if not _runner_context_has_lease(context):
        return
    controller = getattr(context, "runner_owned_controller", None)
    if controller is None:
        return
    executed: list[str] = []
    errors: list[str] = []
    for action in request.get("runnerActions", []):
        check_cancelled()
        try:
            if action == ACTION_EXIT_GAME.value:
                close_game = getattr(controller, "close_current_app", None)
                if not callable(close_game):
                    raise RuntimeError("runner controller lacks close_current_app")
                close_game()
                executed.append(action)
            elif action == ACTION_EXIT_EMULATOR.value:
                close_emulator = getattr(controller, "close_simulator", None)
                if not callable(close_emulator):
                    raise RuntimeError("runner controller lacks close_simulator")
                close_emulator()
                executed.append(action)
        except Exception as error:
            errors.append(f"{action}: {error}")
            log.warning("Runner 完成设备动作失败：%s", error)
    if ACTION_EXIT_EMULATOR.value in executed:
        request["deviceDisposition"] = "emulator_closed"
    elif ACTION_EXIT_GAME.value in executed:
        request["deviceDisposition"] = "game_closed"
    if errors:
        request["deviceActionErrors"] = errors


def get_after_completion_request(
    *,
    run_id: str | None = None,
    outcome: str = "completed",
    forced: bool = False,
    context: object | None = None,
) -> dict[str, object]:
    """Return a serialisable completion-action request for RunnerHost.

    This is deliberately side-effect free until an injected Runner controller is
    present.  In particular it never invokes notifications, power actions, or
    exit-aalc from the task process.
    """

    actions, power_action = get_after_completion_config()
    request = _serialise_after_completion_request(
        actions,
        power_action,
        run_id=run_id,
        outcome=outcome,
        forced=forced,
    )
    if outcome == "completed" and not forced:
        _execute_runner_device_actions(request, context)
    else:
        request["requestedActions"] = list(request["actions"])
        request["requestedPowerAction"] = request["powerAction"]
        request["actions"] = []
        request["sidecarActions"] = []
        request["powerAction"] = "none"
        request["runnerActions"] = []
        request["requiresDeviceLease"] = False
    return request


@begin_and_finish_time_log(task_name="一次经验本")
# 一次经验本的过程
def onetime_EXP_process(combat_count: int = 1):
    if cfg.targeted_teaming_EXP:
        team = cfg.get_value(f"EXP_day_{calculate_the_teams()}")
    else:
        team = cfg.daily_teams
    if EXP_luxcavation(combat_count) is False:
        return False
    select_battle_team(team)
    if battle.to_battle() is False:
        return False
    if battle.fight(combat_count=combat_count) is False:
        return False
    back_init_menu()
    make_enkephalin_module()
    mediator.task_completed.emit("exp", combat_count)
    return True


@begin_and_finish_time_log(task_name="一次纽本")
# 一次纽本的过程
def onetime_thread_process(combat_count: int = 1):
    if cfg.targeted_teaming_thread:
        team = cfg.get_value(f"thread_day_{get_day_of_week()}")
    else:
        team = cfg.daily_teams
    if thread_luxcavation(combat_count) is False:
        return False
    select_battle_team(team)
    if battle.to_battle() is False:
        return False
    if battle.fight(combat_count=combat_count) is False:
        return False
    back_init_menu()
    make_enkephalin_module()
    mediator.task_completed.emit("thread", combat_count)
    return True


@begin_and_finish_time_log(task_name="一次镜牢")
# 一次镜牢的过程
def onetime_mir_process(team_setting: TeamSetting, team_num: int):
    check_cancelled()
    # 实时检查是否需要切换到困难镜牢
    if cfg.auto_hard_mirror and check_hard_mirror_time():
        log.info("检测到新的困牢周期，实时切换困难镜牢，设置困牢次数为3")
        cfg.set_value("last_auto_change", datetime.now().timestamp())
        cfg.set_value("hard_mirror", True)
        cfg.set_value("hard_mirror_chance", 3)

    # 进行一次镜牢
    try:
        mirror_adventure = Mirror(team_setting, team_num)
        if mirror_adventure.run():
            completion_stats = mirror_adventure.last_completion_stats or {}
            del mirror_adventure
            mirror_adventure = None
            back_init_menu()
            make_enkephalin_module()
            return completion_stats
        else:
            return None
    except userStopError:
        raise
    except Exception as e:
        log.exception(f"镜牢行动出错: {e}")
        return None


def to_get_reward():
    if cfg.set_get_prize == 0:
        back_init_menu()
        get_pass_prize()
        back_init_menu()
        get_mail_prize()
        back_init_menu()
    elif cfg.set_get_prize == 1:
        back_init_menu()
        get_pass_prize()
        back_init_menu()
    else:
        back_init_menu()
        get_mail_prize()
        back_init_menu()


def init_game():
    log.debug("初始化游戏")
    # When GPUI has selected a target, reuse the already-established runtime
    # session instead of silently rebuilding the legacy default device from
    # cfg.simulator/cfg.simulator_port.  The old configuration-driven path is
    # retained for desktop and command-line compatibility.
    from module.device_manager import get_device_manager

    device_manager = get_device_manager()
    selected_session = device_manager.active_session
    if selected_session is None and _runner_mode_enabled():
        from module.device_manager import DeviceError

        raise DeviceError("Runner 没有私有设备 session，拒绝按旧配置创建或切换设备")
    if selected_session is not None:
        auto.init_input(session=selected_session)
        if selected_session.target.kind in ("mumu", "adb"):
            controller = selected_session.controller
            if controller is None:
                raise RuntimeError("当前设备会话缺少模拟器控制器")
            controller.start_game()
        else:
            if not screen.handle.hwnd and not screen.init_handle():
                raise withOutGameWinError("未找到已选择的 Limbus Company 游戏窗口")
            if cfg.set_windows:
                screen.set_win()
        return

    if cfg.simulator:
        if cfg.simulator_type == 0:
            mumu_instance_number = 0
            if cfg.simulator_port == 0 and cfg.mumu_instance_number == -1:
                log.info("未设置模拟器端口或实例编号，使用默认mumu模拟器")
            elif cfg.simulator_port != 0:
                if cfg.simulator_port == 16384 or (cfg.simulator_port - 16384) % 32 == 0:
                    mumu_instance_number = 0 if cfg.simulator_port == 16384 else (cfg.simulator_port - 16384) // 32
                    log.debug(f"使用mumu模拟器实例号为 {mumu_instance_number}")
                else:
                    log.info("设置的模拟器端口非常用默认端口，使用默认mumu模拟器")
            elif cfg.mumu_instance_number != -1:
                mumu_instance_number = cfg.mumu_instance_number
            log.debug(
                f"init_game: 模拟器类型=Mumu, 实例编号={mumu_instance_number}, "
                f"simulator_port={cfg.simulator_port}, mumu_instance_number={cfg.mumu_instance_number}"
            )
            from module.automation.input_handlers.simulator.mumu_control import (
                MumuControl,
            )

            MumuControl(instance_number=mumu_instance_number)
        else:
            from module.automation.input_handlers.simulator.simulator_control import (
                SimulatorControl,
            )

            # 启动时先清理旧连接
            SimulatorControl.clean_connect()
            SimulatorControl()
    auto.init_input()
    if cfg.simulator:
        if cfg.simulator_type == 0:
            from module.automation.input_handlers.simulator.mumu_control import (
                MumuControl,
            )

            MumuControl.connection_device.start_game()
        else:
            from module.automation.input_handlers.simulator.simulator_control import (
                SimulatorControl,
            )

            SimulatorControl.connection_device.start_game()
    else:
        game_process.start_game()
        while not screen.init_handle():
            interruptible_sleep(10)
        if cfg.set_windows:
            screen.set_win()


def _is_simulator_runtime() -> bool:
    from module.device_manager import is_simulator_runtime

    return is_simulator_runtime()


def _warn_if_game_monitor_hdr_enabled() -> None:
    if _is_simulator_runtime() or not bool(cfg.get_value("experimental_hdr_warning", True)):
        return

    hwnd = screen.handle.hwnd
    if not hwnd:
        log.warning("游戏窗口句柄无效，跳过 HDR 检测")
        return

    try:
        hmonitor = win32api.MonitorFromWindow(
            hwnd,
            win32con.MONITOR_DEFAULTTONEAREST,
        )
        info = get_monitor_hdr_info(int(hmonitor))
    except Exception as exc:
        log.warning(f"检测游戏显示器 HDR 状态失败: {exc}")
        return

    if info is None or not info.hdr_enabled:
        return

    acknowledged = Event()
    log.warning("检测到游戏所在显示器已开启 HDR，可能导致图像识别问题")
    mediator.hdr_warning.emit(acknowledged)
    # Legacy sinks acknowledge immediately; the bounded helper keeps a Runner
    # stop request able to wake this path if an event sink is unavailable.
    wait_for_event(acknowledged, 10.0)


def Resonate_with_Ahab():
    random_number = random.randint(1, 4)
    playsound(f"assets/audio/This_is_all_your_fault_{random_number}.mp3", block=False)


def _get_game_rendering_scale() -> int | None:
    """读取非模拟器模式下 Limbus 的渲染比例设置。"""
    try:
        import json
        import winreg

        root = winreg.HKEY_CURRENT_USER
        sub_key = r"Software\ProjectMoon\LimbusCompany"
        value_name = "LocalSave.LocalGameOptionData_h467498167"
        with winreg.OpenKey(root, sub_key, 0, winreg.KEY_READ) as key:
            raw_data, reg_type = winreg.QueryValueEx(key, value_name)

        if reg_type != winreg.REG_BINARY:
            log.debug(f"游戏设置注册表值类型为 {reg_type}，预期为 REG_BINARY")
            return None

        json_str = raw_data.rstrip(b"\x00").decode("utf-8")
        game_config = json.loads(json_str)
        return game_config.get("_renderingScale")
    except FileNotFoundError:
        log.debug(r"游戏设置注册表路径不存在: HKEY_CURRENT_USER\Software\ProjectMoon\LimbusCompany")
    except PermissionError:
        log.debug("读取游戏设置注册表时权限不足")
    except Exception as e:
        log.debug(f"读取游戏渲染比例失败: {e}")
    return None


def _batch_combat(process_fn, times, max_times):
    """按 max_times 分批执行战斗"""
    if times <= 0:
        return
    if times > max_times:
        once = max_times
        total = times // max_times
        last = times % max_times
    else:
        once = times
        total = 0
        last = times
    for _ in range(total):
        check_cancelled()
        process_fn(once)
    if last > 0:
        check_cancelled()
        process_fn(last)


def _single_combat_run(exp_times, thread_times):
    for _ in range(exp_times):
        check_cancelled()
        onetime_EXP_process()
    for _ in range(thread_times):
        check_cancelled()
        onetime_thread_process()


def Daily_task_wrapper(get_reward=None):
    def wrapper():
        back_init_menu()
        make_enkephalin_module()
        exp_times = cfg.set_EXP_count
        if get_reward and get_reward == "EXP":
            exp_times -= 1
        thread_times = cfg.set_thread_count
        if get_reward and get_reward == "thread":
            thread_times -= 1
        if cfg.config.use_continuous_combat and cfg.use_continuous_combat_select > 0:
            max_times = cfg.use_continuous_combat_select
            _batch_combat(onetime_EXP_process, exp_times, max_times)
            _batch_combat(onetime_thread_process, thread_times, max_times)
        else:
            _single_combat_run(exp_times, thread_times)

    return wrapper


def Buy_enkephalin():
    times = cfg.set_lunacy_to_enkephalin
    back_init_menu()
    lunacy_to_enkephalin(times=times)


def Mirror_task():
    # 判断执行镜牢任务的次数
    mir_times = cfg.set_mirror_count
    if cfg.infinite_dungeons:
        mir_times = 9999
    if cfg.save_rewards and cfg.hard_mirror:
        mir_times = 1
    finish_times = 0
    mediator.mirror_signal.emit(0, mir_times)
    cfg.normalize_and_sync_team_state(persist=False)
    # 开始执行镜牢任务
    while mir_times > 0:
        check_cancelled()
        # 检测配置的队伍能否顺利执行
        useful = False
        hard = bool(cfg.hard_mirror)
        teams_be_select = cfg.get_value("teams_be_select")
        for index in (i for i, t in enumerate(teams_be_select) if t is True):
            team_setting = cfg.config.teams[f"{index + 1}"]
            if team_setting.fixed_team_use is False:
                useful = True
                break
            if team_setting.fixed_team_use_select == 1 and hard is False:
                useful = True
                break
            if team_setting.fixed_team_use_select == 0 and hard is True:
                useful = True
                break
        if useful is False:
            break

        if not cfg.teams_active_queue:
            break

        team_num = cfg.teams_active_queue[0]
        team_setting = cfg.config.teams[f"{team_num}"]
        # 如果该队伍固定了用途，且用途不符合当前情况，将队首队伍轮转到队尾
        if team_setting.fixed_team_use:
            if (team_setting.fixed_team_use_select == 0 and not cfg.hard_mirror) or (
                team_setting.fixed_team_use_select == 1 and cfg.hard_mirror
            ):
                cfg.rotate_team_queue()
                continue
        # 执行一次镜牢任务，根据执行结果进行处理
        mirror_result = onetime_mir_process(team_setting, team_num)
        if mirror_result is not None:
            cfg.rotate_team_queue()
            mir_times -= 1
            mediator.task_completed.emit("mirror", 1, mirror_result)
            if cfg.hard_mirror and cfg.auto_hard_mirror:
                chance = cfg.hard_mirror_chance - 1
                cfg.set_value("hard_mirror_chance", chance)
                if chance == 0:
                    cfg.set_value("hard_mirror", False)

            # 更新进度条
            finish_times += 1
            mediator.mirror_signal.emit(finish_times, mir_times)
            msg = f"已完成 {finish_times} 次镜牢"
            log.info(msg)
            if finish_times == 1 and cfg.re_claim_rewards:  # 完成第一次镜牢后重新领取奖励
                to_get_reward()

    mediator.mirror_bar_kill_signal.emit()
    if cfg.re_claim_rewards and finish_times > 1:
        to_get_reward()


def script_task(*, runner_mode: bool | None = None, runner_context: object | None = None) -> object:
    """Run the configured task sequence and optionally return sidecar actions.

    The default keeps the historical in-process completion behavior.  Runner
    mode returns a JSON-safe request instead of performing process/UI/power
    side effects in the short-lived child.
    """

    if runner_mode is None:
        runner_mode = _runner_mode_enabled()
    start_time = time()
    check_cancelled()
    # 获取（启动）游戏对游戏窗口进行设置
    init_game()
    _warn_if_game_monitor_hdr_enabled()
    simulator_runtime = _is_simulator_runtime()

    if cfg.skip_enkephalin:
        log.info("设置了跳过合成脑啡肽，将不会自动合成\nSet to skip make enkephalin, it will not to do")
    if not simulator_runtime:
        if _get_game_rendering_scale() == 2:
            log.warning("当前游戏渲染比例为低, 可能会导致识别错误, 建议设置为中或更高")
        if cfg.set_win_size == 720:
            log.warning("当前游戏分辨率为1280*720, 可能会导致识别错误或卡死, 建议设置为更高分辨率")

    path_manager.initialize_paths()
    auto.clear_img_cache()
    log.debug(f"初始化图片路径: {path_manager.pic_path}")
    retry_monitor.start()

    if cfg.resonate_with_Ahab and not runner_mode:
        Resonate_with_Ahab()

    # 如果是战斗中，先处理战斗
    get_reward = None
    if auto.click_element("battle/turn_assets.png", take_screenshot=True):
        get_reward = battle.fight()

    task_list = []
    # 执行日常刷本任务
    if cfg.daily_task:
        task_list.append(("daily_task", Daily_task_wrapper(get_reward=get_reward)))

    # 执行奖励领取任务
    if cfg.get_reward:
        task_list.append(("get_reward", to_get_reward))

    # 执行狂气换饼任务
    if cfg.buy_enkephalin:
        task_list.append(("buy_enkephalin", Buy_enkephalin))

    # 执行镜牢任务
    if cfg.mirror:
        task_list.append(("mirror", Mirror_task))

    for task_id, task in task_list:
        check_cancelled()
        mediator.task_started.emit(task_id)
        task()

    if cfg.set_reduce_miscontact and not simulator_runtime:
        # 任务已结束，这里只恢复游戏窗口样式，避免把前台重新切回游戏。
        screen.reset_win(activate=False)

    log.info("脚本任务已经完成")
    if not runner_mode:
        noop("WindowsToast", "AALC 运行结束")
        noop("WindowsToast", "所有任务已完成")
    dt_start = datetime.fromtimestamp(start_time)
    dt_end = datetime.fromtimestamp(time())
    duration = dt_end - dt_start
    secends = duration.total_seconds()
    minutes, seconds = divmod(secends, 60)
    hours, minutes = divmod(minutes, 60)
    run_time = f"{int(hours):02}:{int(minutes):02}:{int(seconds):02}"
    if not runner_mode:
        send_toast(
            "AALC 运行结束",
            ["所有任务已完成", run_time],
            template=TemplateToast.NormalTemplate,
        )
    if cfg.resonate_with_Ahab and not runner_mode:
        Resonate_with_Ahab()

    if runner_mode:
        # Completion actions are serialized for RunnerTaskHost.  Device actions
        # are only attempted when an explicit leased controller was injected;
        # exit_aalc and power actions remain sidecar-owned requests.
        retry_monitor.stop()
        return get_after_completion_request(
            run_id=os.environ.get("AALC_RUN_ID"),
            outcome="completed",
            forced=False,
            context=runner_context,
        )

    should_exit_aalc = False
    actions: list[str] = []
    if platform.system() == "Windows":
        # 收尾动作可能主动关闭游戏或模拟器。先停止截图监控，避免设备消失
        # 被误判为断链并触发自动恢复，重新拉起刚关闭的模拟器。
        retry_monitor.stop()
        actions, power_action = get_after_completion_config()
        try:
            should_exit_aalc = execute_after_completion(actions, power_action)
        except Exception:
            log.exception("脚本结束后的操作失败")

    from module.device_manager import get_device_manager

    device_manager = get_device_manager()
    selected_session = device_manager.active_session
    if selected_session is not None:
        if selected_session.target.kind == "mumu" and ACTION_EXIT_EMULATOR.value in actions:
            device_manager.release_after_task()
    elif cfg.simulator:
        if cfg.simulator_type == 0:
            from module.automation.input_handlers.simulator.mumu_control import (
                MumuControl,
            )

            MumuControl.clean_connect()

    if should_exit_aalc:
        return 0


class my_script_task(Thread):
    """主脚本任务线程。

    基于标准库 threading.Thread 实现（核心层不依赖 Qt）：
    - isRunning() 为 QThread 时代调用点保留的别名；
    - terminate() 只发出协作取消请求，绝不强杀解释器线程。
    """

    def __init__(self, runner_context: object | None = None):
        # 初始化，构造函数
        super().__init__(daemon=True)
        self.mutex = Lock()
        self.cancel_event = Event()
        self.exception: BaseException | None = None
        self.runner_context = runner_context
        self.runner_mode = _runner_mode_enabled() or runner_context is not None
        self.after_completion_request: dict[str, object] | None = None
        self.device_disposition = "restore"

    def isRunning(self) -> bool:
        """兼容旧 QThread 调用点的别名。"""
        return self.is_alive()

    def get_after_completion_request(self) -> dict[str, object] | None:
        """Return the serialisable Runner completion handoff, if successful."""

        return None if self.after_completion_request is None else dict(self.after_completion_request)

    def run(self):
        self.mutex.acquire()
        from core.execution_control import bind_cancel_event, current_cancel_event

        inherited_cancel_event = current_cancel_event()
        owns_cancel_binding = inherited_cancel_event is None
        if owns_cancel_binding:
            bind_cancel_event(self.cancel_event)

        result: object = None
        try:
            result = self._run()
        except (
            ConnectionError,
            userStopError,
            unableToFindTeamError,
            unexpectNumError,
            cannotOperateGameError,
            netWorkUnstableError,
            backMainWinError,
            withOutGameWinError,
            notWaitError,
            withOutPicError,
            withOutAdminError,
        ) as e:
            self.exception = e
        except Exception as e:
            self.exception = e
            log.exception("脚本线程执行失败")
        finally:
            retry_monitor.stop()
            self.mutex.release()
            if owns_cancel_binding:
                bind_cancel_event(None)

        mediator.script_finished.emit()
        return result

    def terminate(self):
        """Request cooperative cancellation without corrupting Python locks."""
        retry_monitor.stop()
        from core.execution_control import request_cancellation

        self.cancel_event.set()
        request_cancellation()

    """def stop(self):
        self.running=False
        self.finished_signal.emit()"""

    def _run(self):
        keep_awake_enabled = bool(cfg.get_value("experimental_keep_screen_awake", False))
        try:
            if keep_awake_enabled:
                apply_power_keep_awake(True)
            ret = script_task(runner_mode=self.runner_mode, runner_context=self.runner_context)
            if self.runner_mode and isinstance(ret, Mapping):
                self.after_completion_request = dict(ret)
                self.device_disposition = str(ret.get("deviceDisposition", "restore"))
                if self.runner_context is not None:
                    after_completion = getattr(self.runner_context, "after_completion", None)
                    if callable(after_completion):
                        after_completion(self.after_completion_request)
                    set_disposition = getattr(self.runner_context, "set_device_disposition", None)
                    if callable(set_disposition):
                        set_disposition(self.device_disposition)
            elif ret == 0:
                mediator.kill_signal.emit()
            return ret
        finally:
            if keep_awake_enabled:
                # 先切回 AALC 再释放线程级防息屏，避免游戏仍持有前台时继续阻止息屏。
                mediator.request_focus.emit()
                try:
                    interruptible_sleep(0.8)  # 覆盖 WinRT toast 异步归还焦点（延迟约 600ms），再释放防息屏
                finally:
                    # 停止请求可能中断上面的等待，但防息屏必须释放；取消信号继续向上传播。
                    apply_power_keep_awake(False)
            auto.clear_img_cache()
