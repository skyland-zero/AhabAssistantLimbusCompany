"""核心层的可插拔翻译钩子。

核心业务代码不依赖具体界面框架。本模块提供一个进程级翻译函数插槽：

- 核心层通过 :func:`tr` 取译文，未注册翻译器时原样返回源文本；
- 桌面前端可以注册自己的翻译实现，使核心层产生的文案随界面语言切换。

:func:`noop` 用于保留可提取的翻译文本，运行时直接返回原文。
"""

from __future__ import annotations

from typing import Callable

# 翻译器签名: (domain, source_text) -> translated_text
Translator = Callable[[str, str], str]

_translator: Translator | None = None


def register_translator(translator: Translator | None) -> None:
    """注册/替换全局翻译器；传 None 恢复为直通模式。"""
    global _translator
    _translator = translator


def tr(domain: str, text: str) -> str:
    """翻译 source_text；未注册翻译器或无译文时返回原文。"""
    if _translator is None or not text:
        return text
    try:
        return _translator(domain, text)
    except Exception:
        # 翻译失败不应影响业务流程
        return text


def noop(_domain: str, text: str) -> str:
    """标记可翻译文本，运行时返回原文。"""
    return text
