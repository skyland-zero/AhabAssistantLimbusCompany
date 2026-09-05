import logging

from module.logger.my_log import UILogDispatcher, UILogHandler


def test_ui_log_handler_exposes_info_but_filters_debug() -> None:
    dispatcher = UILogDispatcher()
    handler = UILogHandler(dispatcher)
    handler.setFormatter(logging.Formatter("%(message)s"))
    logger = logging.getLogger("AALC.test.ui_log_handler")
    logger.handlers.clear()
    logger.propagate = False
    logger.setLevel(logging.DEBUG)
    logger.addHandler(handler)

    try:
        logger.debug("debug-only")
        logger.info("gui-visible")
        assert dispatcher.snapshot() == ["gui-visible"]
    finally:
        logger.removeHandler(handler)

