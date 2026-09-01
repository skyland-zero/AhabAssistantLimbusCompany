from module.automation.automation import Automation
from module.automation.automation import TextMatchResult as TextMatchResult
from module.config import cfg

auto = Automation(cfg.get_value("game_title_name"))
