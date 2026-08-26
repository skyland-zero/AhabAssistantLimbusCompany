using System;
using System.Runtime.CompilerServices;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Animation;
using Avalonia.Controls;
using Avalonia.Input;
using AhabAssistant.Avalonia.Services;

namespace AhabAssistant.Avalonia.Controls;

/// <summary>Presentation mode used by <see cref="MotionVisibility"/>.</summary>
public enum MotionVisibilityMode
{
    /// <summary>Only change opacity.</summary>
    Fade,

    /// <summary>Fade while moving a short distance vertically.</summary>
    Slide,

    /// <summary>A restrained fade and shorter movement intended for reveals.</summary>
    Reveal,
}

/// <summary>
/// A content control whose logical visibility is controlled by <see cref="IsShown"/>.
/// Hiding keeps the control visible and non-interactive until the exit animation ends.
/// </summary>
public sealed class MotionVisibility : ContentControl
{
    /// <summary>
    /// Attached visibility flag for controls that cannot be wrapped in a
    /// MotionVisibility element. The attached property's XAML name is
    /// MotionVisibility.IsVisible; the real Control.IsVisible value is changed
    /// only after an exit animation has completed.
    /// </summary>
    public new static readonly AttachedProperty<bool> IsVisibleProperty =
        AvaloniaProperty.RegisterAttached<MotionVisibility, Control, bool>(
            "IsVisible",
            true);

    /// <summary>Attached animation mode used with <see cref="IsVisibleProperty"/>.</summary>
    public static readonly AttachedProperty<MotionVisibilityMode> ModeProperty =
        AvaloniaProperty.RegisterAttached<MotionVisibility, Control, MotionVisibilityMode>(
            "Mode",
            MotionVisibilityMode.Fade);

    public static readonly StyledProperty<bool> IsShownProperty =
        AvaloniaProperty.Register<MotionVisibility, bool>(nameof(IsShown), true);

    public static readonly StyledProperty<TimeSpan> DurationProperty =
        AvaloniaProperty.Register<MotionVisibility, TimeSpan>(
            nameof(Duration),
            UiMotion.StandardDuration);

    public static readonly StyledProperty<double> SlideDistanceProperty =
        AvaloniaProperty.Register<MotionVisibility, double>(nameof(SlideDistance), 8d);

    private CancellationTokenSource? _visibilityCancellation;
    private bool _capturedHitTestVisibility;
    private bool _hasCapturedHitTestVisibility;
    private bool _hasAnimatedInitialAppearance;
    private int _visibilityVersion;
    private static readonly ConditionalWeakTable<Control, AttachedVisibilityState> AttachedStates = new();

    static MotionVisibility()
    {
        IsShownProperty.Changed.AddClassHandler<MotionVisibility>(
            (control, change) => control.OnIsShownChanged(change.NewValue is true));
        IsVisibleProperty.Changed.AddClassHandler<Control>(
            (control, change) => GetAttachedState(control).SetShown(change.NewValue is true));
        ModeProperty.Changed.AddClassHandler<Control>(
            (control, change) => GetAttachedState(control).SetMode((MotionVisibilityMode)change.NewValue!));
    }

    /// <summary>Gets the attached animated visibility flag.</summary>
    public static bool GetIsVisible(Control target) =>
        target.GetValue(IsVisibleProperty);

    /// <summary>Sets the attached animated visibility flag.</summary>
    public static void SetIsVisible(Control target, bool value) =>
        target.SetValue(IsVisibleProperty, value);

    /// <summary>Gets the attached animation mode.</summary>
    public static MotionVisibilityMode GetMode(Control target) =>
        target.GetValue(ModeProperty);

    /// <summary>Sets the attached animation mode.</summary>
    public static void SetMode(Control target, MotionVisibilityMode value) =>
        target.SetValue(ModeProperty, value);

    /// <summary>Gets or sets whether the control should be shown.</summary>
    public bool IsShown
    {
        get => GetValue(IsShownProperty);
        set => SetValue(IsShownProperty, value);
    }

    /// <summary>Gets or sets the visibility animation mode.</summary>
    public MotionVisibilityMode Mode
    {
        get => GetValue(ModeProperty);
        set => SetValue(ModeProperty, value);
    }

    /// <summary>Gets or sets the visibility animation duration.</summary>
    public TimeSpan Duration
    {
        get => GetValue(DurationProperty);
        set => SetValue(DurationProperty, value);
    }

    /// <summary>Gets or sets the slide/reveal distance in device-independent pixels.</summary>
    public double SlideDistance
    {
        get => GetValue(SlideDistanceProperty);
        set => SetValue(SlideDistanceProperty, value);
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);

        // IsShown is commonly initialized to true in an item template, so no
        // property-change notification is raised for the first visual attach.
        if (_hasAnimatedInitialAppearance)
        {
            return;
        }

        _hasAnimatedInitialAppearance = true;
        if (!IsShown)
        {
            ApplyFinalState(false);
            return;
        }

        OnIsShownChanged(true);
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        CancelVisibilityAnimation();
        base.OnDetachedFromVisualTree(e);
    }

    private void OnIsShownChanged(bool shown)
    {
        CancelVisibilityAnimation();

        var version = ++_visibilityVersion;
        var cancellation = new CancellationTokenSource();
        _visibilityCancellation = cancellation;

        if (!shown)
        {
            CaptureAndDisableHitTesting();
            SetCurrentValue(Visual.IsVisibleProperty, true);
        }
        else
        {
            SetCurrentValue(Visual.IsVisibleProperty, true);
            RestoreHitTesting();
        }

        var canAnimate = UiMotion.CanAnimate(this, Duration);
        if (!canAnimate)
        {
            ApplyFinalState(shown);
            CompleteVisibilityAnimation(cancellation);
            return;
        }

        var animation = CreateVisibilityAnimation(shown);
        _ = RunVisibilityAnimationAsync(shown, animation, version, cancellation);
    }

    private Animation CreateVisibilityAnimation(bool shown)
    {
        var fromOpacity = Math.Clamp(Opacity, 0, 1);
        var toOpacity = shown ? 1 : 0;

        if (Mode == MotionVisibilityMode.Fade)
        {
            return UiMotion.CreateOpacityAnimation(
                fromOpacity,
                toOpacity,
                Duration,
                shown);
        }

        var currentTranslation = UiMotion.GetTranslation(RenderTransform);
        var distance = Math.Abs(SlideDistance);
        if (Mode == MotionVisibilityMode.Reveal)
        {
            distance *= 0.5;
        }

        var enteringY = distance;
        var exitingY = -distance;
        var fromTransform = UiMotion.CreateTranslation(currentTranslation.X, currentTranslation.Y);
        var toTransform = shown
            ? UiMotion.CreateTranslation(0, 0)
            : UiMotion.CreateTranslation(0, exitingY);

        if (shown && Math.Abs(currentTranslation.Y) < 0.001)
        {
            fromTransform = UiMotion.CreateTranslation(0, enteringY);
            SetCurrentValue(RenderTransformProperty, fromTransform);
        }

        return UiMotion.CreateOpacityAndTranslationAnimation(
            fromOpacity,
            toOpacity,
            fromTransform,
            toTransform,
            Duration,
            shown);
    }

    private async Task RunVisibilityAnimationAsync(
        bool shown,
        Animation animation,
        int version,
        CancellationTokenSource cancellation)
    {
        try
        {
            await UiMotion.RunAsync(this, animation, cancellation.Token).ConfigureAwait(true);
        }
        catch (OperationCanceledException)
        {
            return;
        }
        catch
        {
            // If a target leaves the visual tree during a transition, complete safely.
        }

        if (version == _visibilityVersion &&
            ReferenceEquals(_visibilityCancellation, cancellation) &&
            IsShown == shown)
        {
            ApplyFinalState(shown);
        }

        CompleteVisibilityAnimation(cancellation);
    }

    private void CancelVisibilityAnimation()
    {
        var cancellation = _visibilityCancellation;
        _visibilityCancellation = null;
        if (cancellation is null)
        {
            return;
        }

        try
        {
            cancellation.Cancel();
        }
        finally
        {
            cancellation.Dispose();
        }
    }

    private void CompleteVisibilityAnimation(CancellationTokenSource cancellation)
    {
        if (!ReferenceEquals(_visibilityCancellation, cancellation))
        {
            // A newer state change or a detach already owns and disposed this
            // source.
            return;
        }

        _visibilityCancellation = null;
        cancellation.Dispose();
    }

    private void ApplyFinalState(bool shown)
    {
        SetCurrentValue(Visual.IsVisibleProperty, true);
        SetCurrentValue(OpacityProperty, shown ? 1d : 0d);

        if (Mode != MotionVisibilityMode.Fade)
        {
            SetCurrentValue(
                RenderTransformProperty,
                UiMotion.CreateTranslation(0, shown ? 0 : -Math.Abs(SlideDistance)));
        }

        if (shown)
        {
            RestoreHitTesting();
        }
        else
        {
            SetCurrentValue(IsHitTestVisibleProperty, false);
            SetCurrentValue(Visual.IsVisibleProperty, false);
        }
    }

    private void CaptureAndDisableHitTesting()
    {
        if (!_hasCapturedHitTestVisibility)
        {
            _capturedHitTestVisibility = IsHitTestVisible;
            _hasCapturedHitTestVisibility = true;
        }

        SetCurrentValue(IsHitTestVisibleProperty, false);
    }

    private void RestoreHitTesting()
    {
        if (!_hasCapturedHitTestVisibility)
        {
            return;
        }

        SetCurrentValue(IsHitTestVisibleProperty, _capturedHitTestVisibility);
        _hasCapturedHitTestVisibility = false;
    }

    private static AttachedVisibilityState GetAttachedState(Control target) =>
        AttachedStates.GetValue(
            target,
            control => new AttachedVisibilityState(control, GetMode(control)));

    /// <summary>Animation state for the attached-property form of this control.</summary>
    private sealed class AttachedVisibilityState
    {
        private readonly Control _target;
        private CancellationTokenSource? _cancellation;
        private bool _capturedHitTestVisibility;
        private bool _hasCapturedHitTestVisibility;
        private int _version;
        private MotionVisibilityMode _mode;

        public AttachedVisibilityState(Control target, MotionVisibilityMode mode)
        {
            _target = target;
            _mode = mode;
        }

        public void SetMode(MotionVisibilityMode mode) => _mode = mode;

        public void SetShown(bool shown)
        {
            CancelAnimation();

            var version = ++_version;
            var cancellation = new CancellationTokenSource();
            _cancellation = cancellation;

            if (!shown)
            {
                CaptureAndDisableHitTesting();
                _target.SetCurrentValue(Visual.IsVisibleProperty, true);
            }
            else
            {
                _target.SetCurrentValue(Visual.IsVisibleProperty, true);
                RestoreHitTesting();
            }

            var duration = UiMotion.StandardDuration;
            if (!UiMotion.CanAnimate(_target, duration))
            {
                ApplyFinalState(shown);
                CompleteAnimation(cancellation);
                return;
            }

            var animation = CreateAnimation(shown, duration);
            _ = RunAsync(shown, animation, version, cancellation);
        }

        private Animation CreateAnimation(bool shown, TimeSpan duration)
        {
            var fromOpacity = Math.Clamp(_target.Opacity, 0, 1);
            var toOpacity = shown ? 1 : 0;

            if (_mode == MotionVisibilityMode.Fade)
            {
                return UiMotion.CreateOpacityAnimation(fromOpacity, toOpacity, duration, shown);
            }

            var distance = _mode == MotionVisibilityMode.Reveal ? 4d : 8d;
            var current = UiMotion.GetTranslation(_target.RenderTransform);
            var from = UiMotion.CreateTranslation(current.X, current.Y);
            if (shown && Math.Abs(current.Y) < 0.001)
            {
                from = UiMotion.CreateTranslation(0, distance);
                _target.SetCurrentValue(Visual.RenderTransformProperty, from);
            }

            var to = shown
                ? UiMotion.CreateTranslation(0, 0)
                : UiMotion.CreateTranslation(0, -distance);
            return UiMotion.CreateOpacityAndTranslationAnimation(
                fromOpacity,
                toOpacity,
                from,
                to,
                duration,
                shown);
        }

        private async Task RunAsync(
            bool shown,
            Animation animation,
            int version,
            CancellationTokenSource cancellation)
        {
            try
            {
                await UiMotion.RunAsync(_target, animation, cancellation.Token)
                    .ConfigureAwait(true);
            }
            catch (OperationCanceledException)
            {
                return;
            }
            catch
            {
                // A detached control should still reach a safe final state.
            }

            if (version == _version &&
                ReferenceEquals(_cancellation, cancellation) &&
                GetIsVisible(_target) == shown)
            {
                ApplyFinalState(shown);
            }

            CompleteAnimation(cancellation);
        }

        private void CancelAnimation()
        {
            var cancellation = _cancellation;
            _cancellation = null;
            if (cancellation is null)
            {
                return;
            }

            try
            {
                cancellation.Cancel();
            }
            finally
            {
                cancellation.Dispose();
            }
        }

        private void CompleteAnimation(CancellationTokenSource cancellation)
        {
            if (!ReferenceEquals(_cancellation, cancellation))
            {
                // A newer state change already cancelled and disposed it.
                return;
            }

            _cancellation = null;
            cancellation.Dispose();
        }

        private void ApplyFinalState(bool shown)
        {
            _target.SetCurrentValue(Visual.IsVisibleProperty, true);
            _target.SetCurrentValue(Visual.OpacityProperty, shown ? 1d : 0d);

            if (_mode != MotionVisibilityMode.Fade)
            {
                _target.SetCurrentValue(
                    Visual.RenderTransformProperty,
                    UiMotion.CreateTranslation(0, shown ? 0 : -(_mode == MotionVisibilityMode.Reveal ? 4d : 8d)));
            }

            if (shown)
            {
                RestoreHitTesting();
            }
            else
            {
                _target.SetCurrentValue(InputElement.IsHitTestVisibleProperty, false);
                _target.SetCurrentValue(Visual.IsVisibleProperty, false);
            }
        }

        private void CaptureAndDisableHitTesting()
        {
            if (!_hasCapturedHitTestVisibility)
            {
                _capturedHitTestVisibility = _target.IsHitTestVisible;
                _hasCapturedHitTestVisibility = true;
            }

            _target.SetCurrentValue(InputElement.IsHitTestVisibleProperty, false);
        }

        private void RestoreHitTesting()
        {
            if (!_hasCapturedHitTestVisibility)
            {
                return;
            }

            _target.SetCurrentValue(
                InputElement.IsHitTestVisibleProperty,
                _capturedHitTestVisibility);
            _hasCapturedHitTestVisibility = false;
        }
    }
}
