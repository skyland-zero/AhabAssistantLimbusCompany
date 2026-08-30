"""Core notification package with lazy Windows Toast compatibility exports.

WxPusher is headless and cross-platform, so importing its service must not
eagerly load the optional Windows Toast implementation or its platform-only
dependencies.  The legacy Toast names remain available through ``__getattr__``.
"""

_TOAST_EXPORTS = {"APPID", "APPNAME", "ICONPATH", "TemplateToast", "send_toast", "unregister_toast"}
_WXPUSHER_EXPORTS = {
    "NotificationMessage",
    "NotificationQueue",
    "NotificationSender",
    "NotificationService",
    "WxPusherClient",
    "WxPusherError",
}


def __getattr__(name: str):
    if name in _TOAST_EXPORTS:
        from . import toast

        return getattr(toast, name)
    if name in _WXPUSHER_EXPORTS:
        from . import wxpusher

        return getattr(wxpusher, name)
    raise AttributeError(name)


__all__ = sorted(_TOAST_EXPORTS | _WXPUSHER_EXPORTS)
