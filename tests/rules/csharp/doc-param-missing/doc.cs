public class Api
{
    /// <summary>Adds.</summary>
    /// <param name="a">First.</param>
    public int Add(int a, int b) => a + b;
//                            ^ doc-param-missing
    /// <inheritdoc/>
    public int Sub(int a) => a;
    /// <summary>Private.</summary>
    private int Hidden(int a) => a;
    public int NoDoc(int a) => a;
}
