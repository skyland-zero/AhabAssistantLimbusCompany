using System;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Animation;
using Avalonia.Animation.Easings;
using Avalonia.Media;
using Avalonia.Styling;
using Avalonia.VisualTree;

namespace AhabAssistant.Avalonia.Services;

/// <summary>
/// Shared timing and animation helpers for the lightweight UI motion layer.
/// </summary>
public static class UiMotion
{
    /// <summary>Short feedback transition duration.</summary>
    public static readonly TimeSpan FastDuration = TimeSpan.FromMilliseconds(120);

    /// <summary>Default content and visibility transition duration.</summary>
    public static readonly TimeSpan StandardDuration = TimeSpan.FromMilliseconds(180);

    /// <summary>Default expand/reveal transition duration.</summary>
    public static readonly TimeSpan ExpandDuration = TimeSpan.FromMilliseconds(220);

    /// <summary>Default window entrance transition duration.</summary>
    public static readonly TimeSpan WindowOpenDuration = TimeSpan.FromMilliseconds(240);

    /// <summary>Default window exit transition duration.</summary>
    public static readonly TimeSpan WindowCloseDuration = TimeSpan.FromMilliseconds(160);

    private static bool _isEnabled = true;

    /// <summary>
    /// Gets or sets whether new motion should be played. Setting this to false
    /// makes motion controls apply their final state immediately.
    /// </summary>
    public static bool IsEnabled
    {
        get => _isEnabled;
        set
        {
            if (_isEnabled == value)
            {
                return;
            }

            _isEnabled = value;
            IsEnabledChanged?.Invoke(null, EventArgs.Empty);
        }
    }

    /// <summary>Raised when <see cref="IsEnabled"/> changes.</summary>
    public static event EventHandler? IsEnabledChanged;

    internal static bool CanAnimate(Visual target, TimeSpan duration, bool requested = true)
    {
        return requested &&
               IsEnabled &&
               duration > TimeSpan.Zero &&
               target.IsAttachedToVisualTree();
    }

    internal static TimeSpan NormalizeDuration(TimeSpan duration)
    {
        return duration > TimeSpan.Zero ? duration : TimeSpan.Zero;
    }

    internal static TranslateTransform CreateTranslation(double x, double y)
    {
        return new TranslateTransform(x, y);
    }

    internal static (double X, double Y) GetTranslation(ITransform? transform)
    {
        if (transform is TranslateTransform translation)
        {
            return (translation.X, translation.Y);
        }

        if (transform is null)
        {
            return (0, 0);
        }

        var matrix = transform.Value;
        return (matrix.M31, matrix.M32);
    }

    internal static Animation CreateOpacityAnimation(
        double fromOpacity,
        double toOpacity,
        TimeSpan duration,
        bool entering)
    {
        var animation = CreateAnimation(duration, entering ? new CubicEaseOut() : new CubicEaseIn());
        animation.Children.Add(CreateFrame(
            0,
            new Setter(Visual.OpacityProperty, ClampOpacity(fromOpacity))));
        animation.Children.Add(CreateFrame(
            1,
            new Setter(Visual.OpacityProperty, ClampOpacity(toOpacity))));
        return animation;
    }

    internal static Animation CreateOpacityAndTranslationAnimation(
        double fromOpacity,
        double toOpacity,
        ITransform fromTransform,
        ITransform toTransform,
        TimeSpan duration,
        bool entering)
    {
        var animation = CreateAnimation(duration, entering ? new CubicEaseOut() : new CubicEaseIn());
        animation.Children.Add(CreateFrame(
            0,
            new Setter(Visual.OpacityProperty, ClampOpacity(fromOpacity)),
            new Setter(Visual.RenderTransformProperty, fromTransform)));
        animation.Children.Add(CreateFrame(
            1,
            new Setter(Visual.OpacityProperty, ClampOpacity(toOpacity)),
            new Setter(Visual.RenderTransformProperty, toTransform)));
        return animation;
    }

    internal static async Task RunAsync(
        Visual target,
        Animation animation,
        CancellationToken cancellationToken = default)
    {
        if (!CanAnimate(target, animation.Duration))
        {
            return;
        }

        await animation.RunAsync(target, cancellationToken).ConfigureAwait(true);
    }

    private static Animation CreateAnimation(TimeSpan duration, Easing easing)
    {
        return new Animation
        {
            Duration = NormalizeDuration(duration),
            Easing = easing,
            FillMode = FillMode.Forward,
            PlaybackBehavior = PlaybackBehavior.Always,
        };
    }

    private static KeyFrame CreateFrame(double cue, params Setter[] setters)
    {
        var frame = new KeyFrame
        {
            Cue = new Cue(Math.Clamp(cue, 0, 1)),
        };

        foreach (var setter in setters)
        {
            frame.Setters.Add(setter);
        }

        return frame;
    }

    private static double ClampOpacity(double opacity)
    {
        return Math.Clamp(opacity, 0, 1);
    }
}
