class Qualify
{
    int x;
    static int s;
    int M() => x + s;
//             ^ qualify-instance-members
    int N() => this.x;
    void P(int x) { x = 1; M(); }
//                         ^ qualify-instance-members
}
