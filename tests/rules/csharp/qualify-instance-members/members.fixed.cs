class Qualify
{
    int x;
    static int s;
    int M() => this.x + s;
    int N() => this.x;
    void P(int x) { x = 1; this.M(); }
}
