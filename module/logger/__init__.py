import logging

# "AALC" logger 的 handler（控制台/文件/UI 面板）由入口构造 module.logger.my_log.Logger() 时挂上。
# 这里只按名取它、不构造，所以业务模块 `from module.logger import log` 不会顺带把 GUI 拉进来。
# Logger 和 UI 日志分发由入口/前端按需 `from module.logger.my_log import ...`。
log = logging.getLogger("AALC")

__all__ = ["log"]
