using Avalonia;
using Avalonia.Media;
using Avalonia.Media.Transformation;

public class Test {
    public static void Main() {
        ITransform t = TransformOperations.Parse("translateX(10px)");
        System.Console.WriteLine(t.GetType().Name);
    }
}
