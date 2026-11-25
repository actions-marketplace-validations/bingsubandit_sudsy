public class Api {
  /**
   * Adds.
   * @param a first
   */
  public int add(int a, int b) { return a + b; }
//                          ^ doc-param-missing

  /** Inherits. {@inheritDoc} */
  public int sub(int a) { return a; }

  private int hidden(int a) { return a; }
}
