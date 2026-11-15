"""软件更新检查与下载的核心逻辑。

本模块是核心层实现：不依赖具体界面框架或 UI 层。
- 后台检查线程基于 threading.Thread，结果通过 core.events.Event 回调；
- 文案翻译走 core.i18n 可插拔钩子；
- 下载进度通过 core.events.mediator 对外发布。
"""

import os  # 导入os模块以便操作文件路径
import re
import shutil
import subprocess
import xml.etree.ElementTree as ET
from enum import Enum
from pathlib import Path
from threading import Thread
from urllib.parse import unquote, urlparse

import requests  # 导入requests模块，用于发送HTTP请求
from markdown_it import MarkdownIt
from packaging.version import parse

from core.events import Event, mediator
from core.i18n import tr
from module.config import cfg
from module.logger import log
from module.update.signature import UpdateSignatureError, signature_enforcement_required
from utils.utils import decrypt_string

md_renderer = MarkdownIt("gfm-like", {"html": True})

# 更新日志标题的翻译域，保持配置和文案兼容
TRANSLATE_DOMAIN = "UpdateThread"

# 本 Fork 的 V0.0.x 发布在 skyland-zero 的 GitHub Release 上。上游 KIYI671
# 仓库与 Mirror酱 的 AALC 资源描述的是旧客户端，版本号也属于另一条发布线，
# 因此更新检查只读取本仓库的 GitHub Release，避免把 1.x 当成新版本。
UPDATE_REPO_OWNER = "skyland-zero"
UPDATE_REPO_NAME = "AhabAssistantLimbusCompany"

# 更新检查不走 GitHub API（未认证 60 次/小时，共享 IP 很容易被限流）：
# 稳定版读取 releases.atom 或 releases/latest 的 302 跳转，预览版读取 atom。
_GITHUB_WEB = "https://github.com"
_RELEASE_TAG_MARKER = "/releases/tag/"
_ATOM_NAMESPACE = "{http://www.w3.org/2005/Atom}"

# 国内可访问的 GitHub 代理。检查更新优先读镜像的 releases.atom，镜像全部
# 失败时才直连 GitHub；后续接入下载时也可以复用这些前缀。
UPDATE_MIRRORS = (
    "https://gh-proxy.com",
    "https://ghfast.top",
)


def update_mirror_bases() -> tuple[str, ...]:
    """返回镜像前缀；可用 AALC_UPDATE_MIRROR 以逗号分隔覆盖。"""
    override = os.environ.get("AALC_UPDATE_MIRROR", "").strip()
    if not override:
        return UPDATE_MIRRORS
    return tuple(part.strip().rstrip("/") for part in override.split(",") if part.strip())


class UpdateStatus(Enum):
    """
    定义更新状态的枚举类

    该枚举类用于表示更新操作的三种可能结果状态：
    - SUCCESS 表示更新操作成功
    - UPDATE_AVAILABLE 表示有可用的更新
    - FAILURE 表示更新操作失败
    """

    SUCCESS = 1
    UPDATE_AVAILABLE = 2
    FAILURE = 0


class UpdateThread(Thread):
    """
    更新检查后台线程。

    使用 core.events.Event 通知检查结果；订阅方若涉及 UI 操作，
    需将回调封送回主线程（见 app.event_bridge.connect_queued）。
    """

    def __init__(self, timeout, flag, version=None):
        """
        初始化更新线程。

        参数:
        timeout -- 更新检查请求的超时时间（秒）
        flag -- 是否强制遵循配置项 check_update 来决定是否执行检查
        version -- 当前应用版本；缺省时退回 cfg.version
        """
        super().__init__(daemon=True)
        # 实例级事件：不同检查实例的订阅互不干扰。
        self.updateSignal = Event("update_signal")
        self.timeout = timeout  # 超时时间
        self.flag = flag  # 标志位，用于控制是否执行检查更新
        self.error_msg = ""  # 错误信息
        # 版本以调用方传入的为准：冻结包里 module 的 assets 副本是
        # “DEFAULT VERSION”，只有安装根目录的 version.txt 才是真实版本。
        self.version = version if version is not None else cfg.version

        self.user = UPDATE_REPO_OWNER
        self.repo = UPDATE_REPO_NAME
        self.new_version = ""
        # 记录本次检查后“当前版本是否已追平最新版本”，供资源同步门禁读取。
        self.is_current_version_latest = False

    def _set_version_gate_state(self, version: str):
        """
        刷新版本门禁所需的最新版本信息与比较结果。

        参数:
        version -- 本轮检查得到的最新版本号

        返回:
        当前版本对象与最新版本对象组成的二元组
        """
        current_version = parse(str(self.version).lstrip("Vv"))
        latest_version = parse(version.lstrip("Vv"))
        # 只有两边都解析成功才记录，避免版本号异常时把旧结果当成新版本。
        self.new_version = version
        self.is_current_version_latest = current_version == latest_version
        # 记录本地版本是否为预发布版（alpha/beta/rc），供资源同步门禁跳过测试版。
        self.is_prerelease = current_version.is_prerelease
        return current_version, latest_version

    def _build_release_note_content(self, raw_content: str) -> str:
        """
        统一整理更新日志正文，供不同更新源复用。

        参数:
        raw_content -- 原始的更新日志 Markdown 文本

        返回:
        清理并补充说明后的更新日志文本
        """
        # GitHub Release 不写正文时 API 返回 null；空正文仍然要能完成版本比较。
        content = self.remove_images_from_markdown(raw_content or "")
        return content

    def _emit_version_check_result(self, version: str, current_version, latest_version, content: str) -> None:
        """
        统一根据版本比较结果发出更新状态事件，并在有更新时准备弹窗展示内容。

        参数:
        version -- 本轮检查得到的最新版本号
        current_version -- 当前软件版本解析后的对象
        latest_version -- 远端最新版本解析后的对象
        content -- 已清理完成、可直接渲染的更新日志正文
        """
        # 第一步：若远端版本更高，则准备更新弹窗标题与渲染后的正文。
        if latest_version > current_version:
            self.title = tr(TRANSLATE_DOMAIN, "发现新版本：{Old_version} ——> {New_version}\n更新日志:").format(
                Old_version=self.version, New_version=version
            )
            self.content = "<style>a {color: #586f50; font-weight: bold;}</style>" + md_renderer.render(content)
            self.updateSignal.emit(UpdateStatus.UPDATE_AVAILABLE)
            return

        # 第二步：若当前版本已追平最新版本，则统一发出“已是最新”事件。
        self.updateSignal.emit(UpdateStatus.SUCCESS)

    def run(self) -> None:
        """
        更新线程的主逻辑。
        检查是否有新版本，如果有则发送更新可用事件；否则发送成功事件。
        """
        try:
            # 如果标志位为 False 且配置中的检查更新标志也为 False，则直接返回
            if self.flag and not cfg.get_value("check_update"):
                return

            # 本 Fork 只从自己的 GitHub Release 检查版本；Mirror酱 的 AALC
            # 资源属于上游旧客户端，不能参与这里的版本比较。
            data = self.check_update_info_github()
            version = data["tag_name"]
            # 同步记录本次检查得到的最新版本号与比较结果，供资源同步门禁直接复用。
            current_version, latest_version = self._set_version_gate_state(version)
            # 整理更新日志正文，去掉图片，生成可展示文本。
            content = self._build_release_note_content(data.get("body") or "")

            # 版本比较只依赖 tag；是否带下载资产不影响“有没有新版本”的判断。
            self._emit_version_check_result(version, current_version, latest_version, content)
        except Exception as e:
            # 异常处理，发送失败事件
            self.error_msg = str(e)
            log.error(f"GitHub源检查更新失败:{e}")
            self.updateSignal.emit(UpdateStatus.FAILURE)

    def check_update_info_github(self):
        """
        从 GitHub 网页获取最新发布版本，不占用 API 配额。

        优先读取国内镜像的 ``releases.atom``，镜像全部失败时再直连
        GitHub；稳定版在 Atom 也失败时最后尝试 ``releases/latest`` 的
        302 跳转。Atom 天然按发布时间倒序并包含预发布版本。

        返回:
        含 ``tag_name``、``body``、``assets`` 的发布信息
        """
        try:
            return self._check_update_info_github_atom()
        except Exception as atom_error:
            if not cfg.update_prerelease_enable:
                try:
                    return self._check_update_info_github_redirect()
                except Exception as redirect_error:
                    raise RuntimeError(
                        f"镜像与 GitHub 均无法获取版本：{atom_error}；{redirect_error}"
                    ) from atom_error
            raise

    def _check_update_info_github_redirect(self):
        """稳定版兜底：跟随 releases/latest 的重定向，从 Location 读 tag。"""
        url = f"{_GITHUB_WEB}/{self.user}/{self.repo}/releases/latest"
        response = requests.get(
            url,
            timeout=(4, 12),
            headers=cfg.useragent,
            allow_redirects=False,
        )
        location = response.headers.get("Location", "")
        if response.status_code in (301, 302, 303, 307, 308) and _RELEASE_TAG_MARKER in location:
            tag = unquote(location.split(_RELEASE_TAG_MARKER, 1)[1].split("?", 1)[0].strip("/"))
            if tag:
                return {"tag_name": tag, "body": "", "assets": []}
        response.raise_for_status()
        raise RuntimeError(f"无法从 {url} 解析最新版本")

    def _check_update_info_github_atom(self):
        """按“国内镜像 -> 直连”的顺序解析 releases.atom。"""
        atom_url = f"{_GITHUB_WEB}/{self.user}/{self.repo}/releases.atom"
        urls = [f"{base}/{atom_url}" for base in update_mirror_bases()]
        urls.append(atom_url)
        errors = []
        for url in urls:
            try:
                response = requests.get(url, timeout=(4, 12), headers=cfg.useragent)
                response.raise_for_status()
                return self._latest_release_from_atom(response.text, url)
            except Exception as e:
                errors.append(f"{url}: {e}")
        raise RuntimeError("无法从发布源获取版本：" + "；".join(errors[-2:]))

    def _latest_release_from_atom(self, text: str, source: str):
        """从 Atom 正文里取第一条可用发布，稳定版跳过预发布标签。"""
        root = ET.fromstring(text)
        for entry in root.findall(f"{_ATOM_NAMESPACE}entry"):
            link = entry.find(f"{_ATOM_NAMESPACE}link")
            href = link.get("href", "") if link is not None else ""
            if _RELEASE_TAG_MARKER not in href:
                continue
            tag = unquote(href.split(_RELEASE_TAG_MARKER, 1)[1].split("?", 1)[0].strip("/"))
            if not tag:
                continue
            if not cfg.update_prerelease_enable:
                try:
                    if parse(tag.lstrip("Vv")).is_prerelease:
                        continue
                except Exception:
                    continue
            content = entry.find(f"{_ATOM_NAMESPACE}content")
            body = content.text if content is not None and content.text else ""
            return {"tag_name": tag, "body": body, "assets": []}
        raise RuntimeError(f"发布源中没有可用版本：{source}")

    def check_update_info_github_api(self):
        """
        旧的 GitHub API 实现，只供下载资产路径复用。

        返回:
        最新发布版本的信息（JSON 格式）
        """
        if not cfg.update_prerelease_enable:
            response = requests.get(
                f"https://api.github.com/repos/{self.user}/{self.repo}/releases/latest",
                timeout=10,
                headers=cfg.useragent,
            )
        else:
            response = requests.get(
                f"https://api.github.com/repos/{self.user}/{self.repo}/releases",
                timeout=10,
                headers=cfg.useragent,
            )
        response.raise_for_status()
        # 根据配置决定是否包含预发布版本
        return response.json()[0] if cfg.update_prerelease_enable else response.json()

    def check_update_info_mirrorchyan(self, cdk=""):
        """
        从 mirror酱 获取最新发布版本的信息。

        参数:
        cdk -- Mirror酱 CDK

        返回:
        最新发布版本的信息（JSON 格式）
        """
        if not cfg.update_prerelease_enable:
            response = requests.get(
                f"https://mirrorchyan.com/api/resources/AALC/latest?current_version={self.version}&user_agent={self.repo}&cdk={cdk}",
                timeout=10,
                headers=cfg.useragent,
            )
        else:
            response = requests.get(
                f"https://mirrorchyan.com/api/resources/AALC/latest?current_version={self.version}&user_agent={self.repo}&channel=beta&cdk={cdk}",
                timeout=10,
                headers=cfg.useragent,
            )
        if cdk == "":
            # 返回获取的最新发布版本信息
            return response.json()["data"]
        else:
            return response

    def remove_images_from_markdown(self, markdown_content):
        """
        从 Markdown 内容中移除图片。

        参数:
        markdown_content -- Markdown 格式的文本

        返回:
        移除了图片的 Markdown 文本
        """
        img_pattern = re.compile(r"!\[.*?\]\(.*?\)")
        return img_pattern.sub("", markdown_content)

    def get_download_url_from_assets(self, assets):
        """
        从资产列表中获取 .7z 文件的下载 URL。

        参数:
        assets -- 资产列表（JSON 格式）

        返回:
        .7z 文件的下载 URL，如果没有找到则返回 None
        """
        for asset in assets:
            if asset["name"].endswith(".7z"):
                return asset["browser_download_url"]
        return None

    def get_assets_url(self):
        try:
            if cfg.update_source == "MirrorChyan":
                if cfg.mirrorchyan_cdk == "":
                    self.error_msg = "未设置 Mirror酱 CDK"
                    self.updateSignal.emit(UpdateStatus.FAILURE)
                    return None
                # 符合Mirror酱条件
                cdk = decrypt_string(cfg.mirrorchyan_cdk)
                response = self.check_update_info_mirrorchyan(cdk)
                if response.status_code == 200:
                    mirrorchyan_data = response.json()
                    if mirrorchyan_data["code"] == 0 and mirrorchyan_data["msg"] == "success":
                        url = mirrorchyan_data["data"]["url"]
                        return url
                else:
                    try:
                        mirrorchyan_data = response.json()
                        self.code = mirrorchyan_data["code"]
                        self.error_msg = mirrorchyan_data["msg"]

                        cdk_error_messages = {
                            7001: "Mirror酱 CDK 已过期",
                            7002: "Mirror酱 CDK 错误",
                            7003: "Mirror酱 CDK 今日下载次数已达上限",
                            7004: "Mirror酱 CDK 类型和待下载的资源不匹配",
                            7005: "Mirror酱 CDK 已被封禁",
                        }
                        if self.code in cdk_error_messages:
                            self.error_msg = cdk_error_messages[self.code]
                    except Exception:
                        self.error_msg = "Mirror酱API请求失败"
                    self.updateSignal.emit(UpdateStatus.FAILURE)
                    return None
            else:
                # 下载资产仍走 API；检查版本本身已经改用网页跳转/Atom，不占配额。
                data = self.check_update_info_github_api()
                assets_url = self.get_download_url_from_assets(data["assets"])

                # 如果没有可用的下载 URL，则发送成功信号并返回
                if assets_url is None:
                    self.updateSignal.emit(UpdateStatus.SUCCESS)
                    return
                return assets_url
        except Exception as e:
            # 异常处理，发送失败信号
            log.error(f"更新失败:{e}")
            self.updateSignal.emit(UpdateStatus.FAILURE)


def is_valid_url(url):
    """
    判断给定的URL是否有效。
    该函数通过解析URL的组成部分来验证URL。一个有效的URL应该包含方案（scheme）和网络位置（netloc）。
    参数:
    url (str): 待验证的URL。
    返回:
    bool: 如果URL有效则返回True，否则返回False。
    """
    try:
        result = urlparse(url)
        return result.scheme.lower() == "https" and bool(result.hostname)
    except (TypeError, ValueError):
        return False


def update(assets_url, *, manifest_url=None, signature_url=None):
    """
    从给定的URL下载更新文件到本地。
    :param assets_url : 更新文件的URL。
    """
    if not is_valid_url(assets_url):
        log.error("更新失败：获取的URL无效 ")
        return False

    parsed_url = urlparse(assets_url)
    remote_name = Path(unquote(parsed_url.path)).name
    suffix = Path(remote_name).suffix.lower()
    if suffix not in {".7z", ".zip"}:
        log.error("更新失败：下载文件类型不受支持")
        return False
    file_name = remote_name or f"AALC{suffix}"
    log.info(f"正在下载 {file_name} ...")

    temporary_paths: list[str] = []
    try:
        # Signed releases publish adjacent ``.manifest.json`` and ``.manifest.sig``
        # assets.  Keep the legacy caller shape while allowing a staged rollout.
        archive_stem = Path(file_name).stem
        manifest_url = manifest_url or f"{assets_url.rsplit('/', 1)[0]}/{archive_stem}.manifest.json"
        signature_url = signature_url or f"{assets_url.rsplit('/', 1)[0]}/{archive_stem}.manifest.sig"
        metadata_urls = (manifest_url, signature_url)
        if any(url and not is_valid_url(url) for url in metadata_urls):
            log.error("更新失败：签名清单 URL 无效")
            return False

        with requests.get(assets_url, stream=True, timeout=10) as response:
            response.raise_for_status()
            total_size = int(response.headers.get("content-length", 0))
            max_size = int(os.getenv("AALC_UPDATE_MAX_BYTES", str(2 * 1024 * 1024 * 1024)))
            if total_size > max_size:
                raise ValueError("更新包超过大小限制")
            downloaded = 0
            os.makedirs("update_temp", exist_ok=True)
            file_path = os.path.join("update_temp", file_name)
            temporary_path = f"{file_path}.download"
            temporary_paths.append(temporary_path)

            with open(temporary_path, "wb") as f:
                for chunk in response.iter_content(chunk_size=64 * 1024):
                    if chunk:
                        f.write(chunk)
                        downloaded += len(chunk)
                        if downloaded > max_size:
                            raise ValueError("更新包超过大小限制")
                        if total_size > 0:
                            progress = int(downloaded / total_size * 100)
                            mediator.update_progress.emit(progress)
            os.replace(temporary_path, file_path)

        require_signature = signature_enforcement_required()
        for url, suffix in ((manifest_url, ".manifest.json"), (signature_url, ".manifest.sig")):
            metadata_path = os.path.join("update_temp", f"{archive_stem}{suffix}")
            metadata_temporary_path = f"{metadata_path}.download"
            temporary_paths.append(metadata_temporary_path)
            try:
                with requests.get(url, timeout=10) as metadata_response:
                    metadata_response.raise_for_status()
                    if len(metadata_response.content) > 1024 * 1024:
                        raise ValueError("更新签名清单过大")
                    with open(metadata_temporary_path, "wb") as metadata_file:
                        metadata_file.write(metadata_response.content)
                os.replace(metadata_temporary_path, metadata_path)
            except Exception:
                if require_signature:
                    raise
                log.warning("未获取到可选的更新签名清单：%s", url)

        log.info("下载进度100%")

        if "OCR" in file_name:
            exe_path = os.path.abspath("./assets/binary/7za.exe")
            download_file_path = os.path.join("./update_temp", file_name)
            destination = os.path.abspath("./3rdparty")
            try:
                if os.path.exists(exe_path):
                    subprocess.run(
                        [exe_path, "x", download_file_path, f"-o{destination}", "-aoa"],
                        check=True,
                    )
                else:
                    shutil.unpack_archive(download_file_path, destination)
                log.info("OCR解压完成，请重启AALC")
                return True
            except Exception:
                input("解压失败，按回车键重新解压. . .多次失败请手动下载更新")
                return False
        mediator.download_complete.emit(file_name)
        return True

    except requests.exceptions.RequestException as e:
        log.error(f"下载失败，请检查网络: {e}")
    except OSError as e:
        log.error(f"文件操作失败: {e}")
    except (ValueError, UpdateSignatureError) as e:
        log.error(f"更新包校验失败: {e}")
    finally:
        for temporary_path in temporary_paths:
            try:
                os.remove(temporary_path)
            except FileNotFoundError:
                pass
            except OSError:
                log.debug("清理更新临时文件失败：%s", temporary_path, exc_info=True)
    return False


def start_update_thread(assets_url):
    """
    在单独的线程中启动更新功能。
    :param assets_url: 更新文件的URL。
    """
    thread = Thread(target=update, args=(assets_url,))
    thread.start()
    return thread


def start_update(assert_name):
    source_file = os.path.abspath("./AALC Updater.exe")
    environment = os.environ.copy()
    # The updater enforces signatures by default; carry an explicit opt-out
    # through the detached process instead of relying on inherited state.
    environment.setdefault("AALC_UPDATE_REQUIRE_SIGNATURE", "1")
    subprocess.Popen([source_file, assert_name], creationflags=subprocess.DETACHED_PROCESS, env=environment)
