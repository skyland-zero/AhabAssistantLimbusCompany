using System;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.VisualTree;
using AhabAssistant.Avalonia.Services;

namespace AhabAssistant.Avalonia.Controls;

/// <summary>
/// A Window with a small entrance/exit transition. The actual close is delayed
/// until the exit animation completes, including native and dialog close paths.
/// </summary>
public class MotionWindow : Window
{
    public static readonly StyledProperty<bool> AnimateOnOpenProperty =
        AvaloniaProperty.Register<MotionWindow, bool>(nameof(AnimateOnOpen), true);

    public static readonly StyledProperty<bool> AnimateOnCloseProperty =
        AvaloniaProperty.Register<MotionWindow, bool>(nameof(AnimateOnClose), true);

    public static readonly StyledProperty<bool> CloseOnEscapeProperty =
        AvaloniaProperty.Register<MotionWindow, bool>(nameof(CloseOnEscape), true);

    public static readonly StyledProperty<TimeSpan> OpenDurationProperty =
        AvaloniaProperty.Register<MotionWindow, TimeSpan>(
            nameof(OpenDuration),
            UiMotion.WindowOpenDuration);

    public static readonly StyledProperty<TimeSpan> CloseDurationProperty =
        AvaloniaProperty.Register<MotionWindow, TimeSpan>(
            nameof(CloseDuration),
            UiMotion.WindowCloseDuration);

    public static readonly StyledProperty<double> SlideDistanceProperty =
        AvaloniaProperty.Register<MotionWindow, double>(nameof(SlideDistance), 8d);

    private CancellationTokenSource? _openCancellation;
    private CancellationTokenSource? _closeCancellation;
    private bool _allowClose;
    private bool _closeAnimationStarted;
    private bool _hasPendingDialogResult;
    private object? _pendingDialogResult;

    public MotionWindow()
    {
        // Configure text rasterization for every application window, including
        // dialogs. Use the Avalonia 12 helper methods rather than style setters:
        // the latter cannot target these struct-backed attached properties in
        // Avalonia 12.1.1.
        global::Avalonia.Media.TextOptions.SetTextRenderingMode(
            this,
            OperatingSystem.IsWindows()
                ? TextRenderingMode.SubpixelAntialias
                : TextRenderingMode.Antialias);
        global::Avalonia.Media.TextOptions.SetTextHintingMode(
            this, TextHintingMode.Strong);
        global::Avalonia.Media.TextOptions.SetBaselinePixelAlignment(
            this, BaselinePixelAlignment.Aligned);
    }

    /// <summary>Gets or sets whether an entrance animation is played.</summary>
    public bool AnimateOnOpen
    {
        get => GetValue(AnimateOnOpenProperty);
        set => SetValue(AnimateOnOpenProperty, value);
    }

    /// <summary>Gets or sets whether an exit animation is played.</summary>
    public bool AnimateOnClose
    {
        get => GetValue(AnimateOnCloseProperty);
        set => SetValue(AnimateOnCloseProperty, value);
    }

    /// <summary>Gets or sets whether Escape requests a normal window close.</summary>
    public bool CloseOnEscape
    {
        get => GetValue(CloseOnEscapeProperty);
        set => SetValue(CloseOnEscapeProperty, value);
    }

    /// <summary>Gets or sets the entrance animation duration.</summary>
    public TimeSpan OpenDuration
    {
        get => GetValue(OpenDurationProperty);
        set => SetValue(OpenDurationProperty, value);
    }

    /// <summary>Gets or sets the exit animation duration.</summary>
    public TimeSpan CloseDuration
    {
        get => GetValue(CloseDurationProperty);
        set => SetValue(CloseDurationProperty, value);
    }

    /// <summary>Gets or sets the entrance/exit translation distance.</summary>
    public double SlideDistance
    {
        get => GetValue(SlideDistanceProperty);
        set => SetValue(SlideDistanceProperty, value);
    }

    /// <summary>
    /// Hides <see cref="Window.Close()"/> so callers using MotionWindow retain
    /// the delayed close behavior.
    /// </summary>
    public new void Close()
    {
        RequestBaseClose(hasDialogResult: false, dialogResult: null);
    }

    /// <summary>
    /// Hides <see cref="Window.Close(object)"/> while preserving the dialog result.
    /// </summary>
    public new void Close(object? dialogResult)
    {
        RequestBaseClose(hasDialogResult: true, dialogResult);
    }

    /// <summary>Requests a close and lets the exit animation complete first.</summary>
    public Task RequestCloseAsync()
    {
        RequestBaseClose(hasDialogResult: false, dialogResult: null);
        return Task.CompletedTask;
    }

    /// <summary>Requests a dialog close while preserving its result.</summary>
    public Task RequestCloseAsync(object? dialogResult)
    {
        RequestBaseClose(hasDialogResult: true, dialogResult);
        return Task.CompletedTask;
    }

    protected override void OnOpened(EventArgs e)
    {
        CancelOpenAnimation();

        var animate = AnimateOnOpen && UiMotion.CanAnimate(this, OpenDuration);
        if (animate)
        {
            SetCurrentValue(OpacityProperty, 0d);
            SetCurrentValue(
                RenderTransformProperty,
                UiMotion.CreateTranslation(0, Math.Abs(SlideDistance)));
        }
        else
        {
            SetCurrentValue(OpacityProperty, 1d);
            SetCurrentValue(RenderTransformProperty, UiMotion.CreateTranslation(0, 0));
        }

        base.OnOpened(e);

        if (animate)
        {
            var cancellation = new CancellationTokenSource();
            _openCancellation = cancellation;
            _ = RunOpenAnimationAsync(cancellation);
        }
    }

    protected override void OnClosing(WindowClosingEventArgs e)
    {
        // Let existing Closing subscribers cancel the request before motion is added.
        base.OnClosing(e);

        if (e.Cancel)
        {
            if (!_closeAnimationStarted && !_allowClose)
            {
                _hasPendingDialogResult = false;
                _pendingDialogResult = null;
            }

            return;
        }

        if (_allowClose)
        {
            return;
        }

        if (_closeAnimationStarted)
        {
            e.Cancel = true;
            return;
        }

        if (!ShouldAnimateClose(e))
        {
            return;
        }

        e.Cancel = true;
        _closeAnimationStarted = true;
        CancelOpenAnimation();

        var cancellation = new CancellationTokenSource();
        _closeCancellation = cancellation;
        _ = RunCloseAnimationAsync(cancellation);
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);

        if (!e.Handled && CloseOnEscape && e.Key == Key.Escape)
        {
            e.Handled = true;
            Close();
        }
    }

    private void RequestBaseClose(bool hasDialogResult, object? dialogResult)
    {
        if (_allowClose)
        {
            if (hasDialogResult)
            {
                base.Close(dialogResult);
            }
            else
            {
                base.Close();
            }

            return;
        }

        if (_closeAnimationStarted)
        {
            return;
        }

        _hasPendingDialogResult = hasDialogResult;
        _pendingDialogResult = dialogResult;

        // Calling the framework method first keeps normal Window dialog-result
        // bookkeeping intact. OnClosing cancels this close and starts the exit.
        if (hasDialogResult)
        {
            base.Close(dialogResult);
        }
        else
        {
            base.Close();
        }
    }

    private bool ShouldAnimateClose(WindowClosingEventArgs e)
    {
        if (!AnimateOnClose ||
            !UiMotion.CanAnimate(this, CloseDuration) ||
            !this.IsAttachedToVisualTree())
        {
            return false;
        }

        return e.CloseReason != WindowCloseReason.ApplicationShutdown &&
               e.CloseReason != WindowCloseReason.OSShutdown;
    }

    private async Task RunOpenAnimationAsync(CancellationTokenSource cancellation)
    {
        try
        {
            var animation = UiMotion.CreateOpacityAndTranslationAnimation(
                0,
                1,
                UiMotion.CreateTranslation(0, Math.Abs(SlideDistance)),
                UiMotion.CreateTranslation(0, 0),
                OpenDuration,
                entering: true);
            await UiMotion.RunAsync(this, animation, cancellation.Token).ConfigureAwait(true);
        }
        catch (OperationCanceledException)
        {
            return;
        }
        catch
        {
            // Complete to a usable window if the platform detaches during opening.
        }

        if (ReferenceEquals(_openCancellation, cancellation) && !_closeAnimationStarted)
        {
            SetCurrentValue(OpacityProperty, 1d);
            SetCurrentValue(RenderTransformProperty, UiMotion.CreateTranslation(0, 0));
        }

        CompleteOpenAnimation(cancellation);
    }

    private async Task RunCloseAnimationAsync(CancellationTokenSource cancellation)
    {
        try
        {
            var translation = UiMotion.GetTranslation(RenderTransform);
            var animation = UiMotion.CreateOpacityAndTranslationAnimation(
                Opacity,
                0,
                UiMotion.CreateTranslation(translation.X, translation.Y),
                UiMotion.CreateTranslation(0, -Math.Abs(SlideDistance)),
                CloseDuration,
                entering: false);
            await UiMotion.RunAsync(this, animation, cancellation.Token).ConfigureAwait(true);
        }
        catch (OperationCanceledException)
        {
            CompleteCloseAnimation(cancellation);
            return;
        }
        catch
        {
            // Closing must still complete if an animation cannot be scheduled.
        }

        if (!ReferenceEquals(_closeCancellation, cancellation))
        {
            cancellation.Dispose();
            return;
        }

        SetCurrentValue(OpacityProperty, 0d);
        SetCurrentValue(
            RenderTransformProperty,
            UiMotion.CreateTranslation(0, -Math.Abs(SlideDistance)));

        _allowClose = true;
        try
        {
            if (_hasPendingDialogResult)
            {
                base.Close(_pendingDialogResult);
            }
            else
            {
                base.Close();
            }
        }
        finally
        {
            CompleteCloseAnimation(cancellation);
        }
    }

    private void CancelOpenAnimation()
    {
        var cancellation = _openCancellation;
        _openCancellation = null;
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

    private void CompleteOpenAnimation(CancellationTokenSource cancellation)
    {
        if (!ReferenceEquals(_openCancellation, cancellation))
        {
            // Closing or a later open request already owns and disposed it.
            return;
        }

        _openCancellation = null;
        cancellation.Dispose();
    }

    private void CompleteCloseAnimation(CancellationTokenSource cancellation)
    {
        if (!ReferenceEquals(_closeCancellation, cancellation))
        {
            return;
        }

        _closeCancellation = null;
        cancellation.Dispose();
    }
}
