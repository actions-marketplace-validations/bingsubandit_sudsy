/// <summary>Documented.</summary>
public class Api
{
    private int x;
    public int count;
//             ^^^^^ missing-doc-comment
    /// <summary>Documented.</summary>
    public void Ok() { }
    [Obsolete]
    public void Run() { }
//              ^^^ missing-doc-comment
    internal int Size { get; }
//               ^^^^ missing-doc-comment
    public Api() { }
//         ^^^ missing-doc-comment
    void IDisposable.Dispose() { }
//                   ^^^^^^^ missing-doc-comment
    private class Hidden { public void F() { } }
    public enum Color { Red }
//              ^^^^^ missing-doc-comment
//                      ^^^ missing-doc-comment
    public partial class Part { }
}
interface IFace { void M(); }
//        ^^^^^ missing-doc-comment
//                     ^ missing-doc-comment
