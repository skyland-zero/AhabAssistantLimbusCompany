use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    Div, EntityId, Pixels, ScrollDelta, ScrollHandle, Stateful, Window, point, prelude::*, px,
};

const SMOOTH_SCROLL_TIME_CONSTANT: f32 = 0.095;
const MAX_FRAME_STEP: f32 = 0.05;
const SNAP_DISTANCE: f32 = 0.5;

#[derive(Default)]
struct SmoothScrollState {
    target: Option<gpui::Point<Pixels>>,
    last_written: Option<gpui::Point<Pixels>>,
    last_frame: Option<Instant>,
    running: bool,
}

/// Owns the stable GPUI scroll handle and the state used to animate coarse
/// mouse-wheel events. Pixel-precise trackpad events intentionally continue
/// through GPUI's native scroll listener.
#[derive(Clone)]
pub(crate) struct SmoothScrollController {
    handle: ScrollHandle,
    state: Rc<RefCell<SmoothScrollState>>,
}

impl SmoothScrollController {
    pub(crate) fn new() -> Self {
        Self::with_handle(ScrollHandle::new())
    }

    pub(crate) fn with_handle(handle: ScrollHandle) -> Self {
        Self {
            handle,
            state: Rc::new(RefCell::new(SmoothScrollState::default())),
        }
    }

    pub(crate) fn attach(&self, area: Stateful<Div>) -> Stateful<Div> {
        let handle = self.handle.clone();
        let state = self.state.clone();

        area.track_scroll(&handle)
            .on_scroll_wheel(move |event, window, cx| {
                let ScrollDelta::Lines(lines) = event.delta else {
                    return;
                };

                let delta_y = window.line_height() * lines.y;
                if delta_y == Pixels::ZERO {
                    return;
                }

                let current = handle.offset();
                let max_offset = handle.max_offset();
                let (changed, immediate, should_start) = {
                    let mut state = state.borrow_mut();

                    // A scrollbar drag, a ScrollHandle update, or a layout pass
                    // may have moved the offset outside of this animation loop.
                    // Treat that movement as the new starting point.
                    if state.running
                        && state
                            .last_written
                            .is_some_and(|last_written| last_written != current)
                    {
                        state.target = None;
                        state.last_frame = None;
                        state.running = false;
                    }

                    let previous_target = state.target.unwrap_or(current);
                    let target = target_after_delta(previous_target, delta_y, max_offset);
                    let changed = target != previous_target;
                    let mut immediate = false;
                    let mut should_start = false;

                    if changed {
                        if cx.reduce_motion() {
                            handle.set_offset(target);
                            state.target = None;
                            state.last_frame = None;
                            state.running = false;
                            state.last_written = Some(target);
                            immediate = true;
                        } else {
                            state.target = Some(target);
                            state.last_written = Some(current);
                            if !state.running {
                                state.running = true;
                                state.last_frame = Some(Instant::now());
                                should_start = true;
                            }
                        }
                    }
                    (changed, immediate, should_start)
                };

                if !changed {
                    return;
                }

                // Match Zed's editor behavior: consume the event only when this
                // scroll container can actually move, allowing an ancestor to
                // handle wheel input at the boundary.
                cx.stop_propagation();
                if immediate {
                    cx.notify(window.current_view());
                } else if should_start {
                    schedule_animation_frame(
                        window,
                        handle.clone(),
                        state.clone(),
                        window.current_view(),
                    );
                }
            })
    }
}

fn schedule_animation_frame(
    window: &Window,
    handle: ScrollHandle,
    state: Rc<RefCell<SmoothScrollState>>,
    view: EntityId,
) {
    window.on_next_frame(move |window, cx| {
        let mut schedule_next = false;
        let mut changed = false;

        {
            let mut state = state.borrow_mut();
            if !state.running {
                return;
            }

            let raw_current = handle.offset();
            let max_offset = handle.max_offset();

            // Detect direct mutations made by a scrollbar or a public
            // ScrollHandle API between animation frames.
            if state
                .last_written
                .is_some_and(|last_written| last_written != raw_current)
            {
                state.target = None;
                state.last_frame = None;
                state.running = false;
                state.last_written = Some(raw_current);
                return;
            }

            let current = clamp_offset(raw_current, max_offset);
            let Some(raw_target) = state.target else {
                state.running = false;
                state.last_frame = None;
                return;
            };
            let target = clamp_offset(raw_target, max_offset);
            state.target = Some(target);

            let now = Instant::now();
            let delta_time = state
                .last_frame
                .map(|last_frame| now.saturating_duration_since(last_frame))
                .unwrap_or(Duration::from_millis(16))
                .as_secs_f32()
                .min(MAX_FRAME_STEP);
            state.last_frame = Some(now);

            let alpha = 1.0 - (-delta_time / SMOOTH_SCROLL_TIME_CONSTANT).exp();
            let mut next = interpolate(current, target, alpha);
            if is_close(next, target) {
                next = target;
                state.target = None;
                state.last_frame = None;
                state.running = false;
            } else {
                schedule_next = true;
            }

            next = clamp_offset(next, max_offset);
            if next != raw_current {
                handle.set_offset(next);
                changed = true;
            }
            state.last_written = Some(next);
        }

        if changed {
            cx.notify(view);
        }
        if schedule_next {
            schedule_animation_frame(window, handle, state, view);
        }
    });
}

fn clamp_offset(
    offset: gpui::Point<Pixels>,
    max_offset: gpui::Point<Pixels>,
) -> gpui::Point<Pixels> {
    point(
        offset.x.clamp(-max_offset.x, px(0.)),
        offset.y.clamp(-max_offset.y, px(0.)),
    )
}

fn target_after_delta(
    current: gpui::Point<Pixels>,
    delta_y: Pixels,
    max_offset: gpui::Point<Pixels>,
) -> gpui::Point<Pixels> {
    clamp_offset(point(current.x, current.y + delta_y), max_offset)
}

fn interpolate(
    current: gpui::Point<Pixels>,
    target: gpui::Point<Pixels>,
    alpha: f32,
) -> gpui::Point<Pixels> {
    point(
        px(current.x.as_f32() + (target.x.as_f32() - current.x.as_f32()) * alpha),
        px(current.y.as_f32() + (target.y.as_f32() - current.y.as_f32()) * alpha),
    )
}

fn is_close(current: gpui::Point<Pixels>, target: gpui::Point<Pixels>) -> bool {
    (current.x.as_f32() - target.x.as_f32()).abs() <= SNAP_DISTANCE
        && (current.y.as_f32() - target.y.as_f32()).abs() <= SNAP_DISTANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_scroll_offsets_to_gpui_bounds() {
        let offset = clamp_offset(point(px(20.), px(-120.)), point(px(80.), px(100.)));
        assert_eq!(offset, point(px(0.), px(-100.)));
    }

    #[test]
    fn interpolation_moves_towards_target() {
        let current = point(px(0.), px(0.));
        let target = point(px(0.), px(-100.));
        assert_eq!(interpolate(current, target, 0.25), point(px(0.), px(-25.)));
    }

    #[test]
    fn wheel_targets_accumulate_before_the_animation_catches_up() {
        let max_offset = point(px(0.), px(100.));
        let first = target_after_delta(point(px(0.), px(0.)), px(-70.), max_offset);
        let second = target_after_delta(first, px(-70.), max_offset);

        assert_eq!(first, point(px(0.), px(-70.)));
        assert_eq!(second, point(px(0.), px(-100.)));
    }

    #[test]
    fn close_positions_snap() {
        assert!(is_close(point(px(0.), px(-99.7)), point(px(0.), px(-100.))));
        assert!(!is_close(point(px(0.), px(-98.)), point(px(0.), px(-100.))));
    }
}
