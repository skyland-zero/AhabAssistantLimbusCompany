using System;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.VisualTree;
using AhabAssistant.Avalonia.Services;

namespace AhabAssistant.Avalonia.Controls;

/// <summary>Direction used by <see cref="MotionContentHost"/> when sliding content.</summary>
public enum MotionDirection
{
    /// <summary>Enter from the right and leave to the left.</summary>
    Forward,

    /// <summary>Enter from the left and leave to the right.</summary>
    Backward,

    /// <summary>Enter from below and leave above.</summary>
    Up,

    /// <summary>Enter from above and leave below.</summary>
    Down,

    /// <summary>Use a cross-fade without translation.</summary>
    None,
}

/// <summary>
/// A small content host that cross-fades and slides between controls.
/// Use <see cref="TransitionTo"/> to change the hosted control.
/// </summary>
public sealed class MotionContentHost : ContentControl
{
    public static readonly StyledProperty<TimeSpan> DurationProperty =
        AvaloniaProperty.Register<MotionContentHost, TimeSpan>(
            nameof(Duration),
            UiMotion.StandardDuration);

    public static readonly StyledProperty<double> SlideDistanceProperty =
        AvaloniaProperty.Register<MotionContentHost, double>(nameof(SlideDistance), 8d);

    public static readonly DirectProperty<MotionContentHost, Control?> CurrentContentProperty =
        AvaloniaProperty.RegisterDirect<MotionContentHost, Control?>(
            nameof(CurrentContent),
            host => host.CurrentContent);

    private readonly Grid _layerHost;
    private CancellationTokenSource? _transitionCancellation;
    private Control? _currentContent;
    private int _transitionVersion;

    public MotionContentHost()
    {
        _layerHost = new Grid
        {
            ClipToBounds = true,
        };

        // The regular ContentControl template still provides theme integration;
        // the template sees this stable layer host while CurrentContent exposes
        // the control that the caller selected.
        base.Content = _layerHost;
    }

    /// <summary>Gets or sets the transition duration.</summary>
    public TimeSpan Duration
    {
        get => GetValue(DurationProperty);
        set => SetValue(DurationProperty, value);
    }

    /// <summary>Gets or sets the horizontal/vertical slide distance in device-independent pixels.</summary>
    public double SlideDistance
    {
        get => GetValue(SlideDistanceProperty);
        set => SetValue(SlideDistanceProperty, value);
    }

    /// <summary>Gets the content currently selected by <see cref="TransitionTo"/>.</summary>
    public Control? CurrentContent => _currentContent;

    /// <summary>
    /// Replaces the current content. A new call cancels the previous transition
    /// and leaves only the newest content in the host.
    /// </summary>
    public void TransitionTo(
        Control? content,
        MotionDirection direction = MotionDirection.Forward,
        bool animate = true)
    {
        var oldContent = _currentContent;
        if (ReferenceEquals(oldContent, content) && HasSingleCurrentSlot(content))
        {
            return;
        }

        CancelTransition();

        var version = ++_transitionVersion;
        var cancellation = new CancellationTokenSource();
        _transitionCancellation = cancellation;

        var oldSlot = FindSlot(oldContent);
        var newSlot = content is null ? null : FindSlot(content);

        if (content is not null && newSlot is null)
        {
            EnsureCanHost(content);
            newSlot = new MotionSlot(content);
            _layerHost.Children.Add(newSlot);
        }

        RemoveStaleSlots(oldSlot, newSlot);

        if (oldSlot is not null)
        {
            oldSlot.IsHitTestVisible = false;
            oldSlot.ZIndex = 0;
        }

        if (newSlot is not null)
        {
            newSlot.IsVisible = true;
            newSlot.IsHitTestVisible = true;
            newSlot.ZIndex = 1;
        }

        SetCurrentContent(content);

        var canAnimate = animate &&
                         UiMotion.CanAnimate(this, Duration) &&
                         (newSlot is null || newSlot.IsAttachedToVisualTree());

        if (!canAnimate)
        {
            CompleteTransition(oldSlot, newSlot);
            CompleteTransitionCancellation(cancellation);
            return;
        }

        var (enterX, enterY) = GetEnterOffset(direction);
        var (exitX, exitY) = (-enterX, -enterY);

        if (newSlot is not null)
        {
            newSlot.Opacity = 0;
            newSlot.RenderTransform = UiMotion.CreateTranslation(enterX, enterY);
        }

        if (oldSlot is not null)
        {
            oldSlot.Opacity = 1;
            oldSlot.RenderTransform = UiMotion.CreateTranslation(0, 0);
        }

        _ = RunTransitionAsync(
            oldSlot,
            newSlot,
            exitX,
            exitY,
            version,
            cancellation);
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        CancelTransition();
        base.OnDetachedFromVisualTree(e);
    }

    private async Task RunTransitionAsync(
        MotionSlot? oldSlot,
        MotionSlot? newSlot,
        double exitX,
        double exitY,
        int version,
        CancellationTokenSource cancellation)
    {
        try
        {
            var animations = new Task[2];
            var animationCount = 0;

            if (oldSlot is not null)
            {
                var oldTranslation = UiMotion.GetTranslation(oldSlot.RenderTransform);
                var oldAnimation = UiMotion.CreateOpacityAndTranslationAnimation(
                    oldSlot.Opacity,
                    0,
                    UiMotion.CreateTranslation(oldTranslation.X, oldTranslation.Y),
                    UiMotion.CreateTranslation(exitX, exitY),
                    Duration,
                    entering: false);
                animations[animationCount++] = UiMotion.RunAsync(
                    oldSlot,
                    oldAnimation,
                    cancellation.Token);
            }

            if (newSlot is not null)
            {
                var newTranslation = UiMotion.GetTranslation(newSlot.RenderTransform);
                var newAnimation = UiMotion.CreateOpacityAndTranslationAnimation(
                    newSlot.Opacity,
                    1,
                    UiMotion.CreateTranslation(newTranslation.X, newTranslation.Y),
                    UiMotion.CreateTranslation(0, 0),
                    Duration,
                    entering: true);
                animations[animationCount++] = UiMotion.RunAsync(
                    newSlot,
                    newAnimation,
                    cancellation.Token);
            }

            if (animationCount > 0)
            {
                await Task.WhenAll(animations.AsSpan(0, animationCount).ToArray())
                    .ConfigureAwait(true);
            }
        }
        catch (OperationCanceledException)
        {
            return;
        }
        catch
        {
            // A detached or re-templated target should fail safe to the final state.
        }

        if (version == _transitionVersion &&
            ReferenceEquals(_transitionCancellation, cancellation))
        {
            CompleteTransition(oldSlot, newSlot);
        }

        CompleteTransitionCancellation(cancellation);
    }

    private void CancelTransition()
    {
        var cancellation = _transitionCancellation;
        _transitionCancellation = null;
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

    private void CompleteTransitionCancellation(CancellationTokenSource cancellation)
    {
        // A newer transition owns its own source and has already cancelled and
        // disposed this one. Only the current owner may dispose it here.
        if (!ReferenceEquals(_transitionCancellation, cancellation))
        {
            return;
        }

        _transitionCancellation = null;
        cancellation.Dispose();
    }

    private void CompleteTransition(MotionSlot? oldSlot, MotionSlot? newSlot)
    {
        if (oldSlot is not null && _layerHost.Children.Contains(oldSlot))
        {
            _layerHost.Children.Remove(oldSlot);
        }

        if (newSlot is not null)
        {
            newSlot.IsVisible = true;
            newSlot.IsHitTestVisible = true;
            newSlot.Opacity = 1;
            newSlot.RenderTransform = UiMotion.CreateTranslation(0, 0);
            newSlot.ZIndex = 0;
        }
        else
        {
            for (var i = _layerHost.Children.Count - 1; i >= 0; i--)
            {
                _layerHost.Children.RemoveAt(i);
            }
        }
    }

    private void RemoveStaleSlots(MotionSlot? oldSlot, MotionSlot? newSlot)
    {
        for (var i = _layerHost.Children.Count - 1; i >= 0; i--)
        {
            var slot = _layerHost.Children[i];
            if (!ReferenceEquals(slot, oldSlot) && !ReferenceEquals(slot, newSlot))
            {
                _layerHost.Children.RemoveAt(i);
            }
        }
    }

    private MotionSlot? FindSlot(Control? content)
    {
        if (content is null)
        {
            return null;
        }

        foreach (var child in _layerHost.Children)
        {
            if (child is MotionSlot slot && ReferenceEquals(slot.HostedContent, content))
            {
                return slot;
            }
        }

        return null;
    }

    private bool HasSingleCurrentSlot(Control? content)
    {
        return content is not null &&
               _layerHost.Children.Count == 1 &&
               FindSlot(content) is not null;
    }

    private void EnsureCanHost(Control content)
    {
        if (content.Parent is not null || content.GetVisualParent() is not null)
        {
            throw new InvalidOperationException(
                "MotionContentHost can only host a control that is not already attached to another visual or logical parent.");
        }
    }

    private void SetCurrentContent(Control? content)
    {
        if (ReferenceEquals(_currentContent, content))
        {
            return;
        }

        SetAndRaise(CurrentContentProperty, ref _currentContent, content);
    }

    private (double X, double Y) GetEnterOffset(MotionDirection direction)
    {
        var distance = Math.Abs(SlideDistance);
        return direction switch
        {
            MotionDirection.Backward => (-distance, 0),
            MotionDirection.Up => (0, distance),
            MotionDirection.Down => (0, -distance),
            MotionDirection.None => (0, 0),
            _ => (distance, 0),
        };
    }

    private sealed class MotionSlot : ContentControl
    {
        public MotionSlot(Control content)
        {
            HostedContent = content;
            base.Content = content;
            HorizontalContentAlignment = HorizontalAlignment.Stretch;
            VerticalContentAlignment = VerticalAlignment.Stretch;
        }

        public Control HostedContent { get; }
    }
}
