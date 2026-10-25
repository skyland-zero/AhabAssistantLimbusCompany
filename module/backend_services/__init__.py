"""Narrow services composed by :class:`module.backend_application.BackendApplication`.

The sidecar's application object remains the composition root for execution,
device and preview lifetime.  Pure or externally bounded concerns live here so
they can be tested without constructing the full runtime.
"""

from __future__ import annotations

from .notifications import NotificationUpdateService
from .resources import ResourceService
from .tasks import TaskConfigService
from .teams import TeamService
from .theme_packs import ThemePackService

__all__ = [
    "NotificationUpdateService",
    "ResourceService",
    "TaskConfigService",
    "TeamService",
    "ThemePackService",
]
