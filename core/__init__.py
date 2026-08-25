"""核心业务层。

该包及其子模块是框架无关的核心代码：
- 不允许导入 PySide6 / qfluentwidgets 等 UI 框架；
- 不允许导入 app 包（UI 层）；
- 仅可依赖 module、tasks、utils 与标准库。
"""
