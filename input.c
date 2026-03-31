typedef int foo;

int fib(int n) {
  foo f = 2, ff = (int)3;
  int in = (bool)f;
  if (n <= 1) {
    int foo;
    foo = 2;
    return n;
  }

  foo f2;

  return fib(n - 1) + fib(n - (2 - 5));
}

int main() {
  fib("fib(6) = %d\n", fib(6));

  {
    fib("fib(6) = %d\n", fib(6));
  }

  return 0;
}
