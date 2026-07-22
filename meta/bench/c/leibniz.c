/* Leibniz partial sum — same work as src/examples/bench/leibniz-loop.lkjml */
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
  long n = 200000;
  if (argc > 1) {
    n = atol(argv[1]);
  }
  double acc = 0.0;
  for (long i = 0; i < n; i++) {
    double term = 1.0 / (2.0 * (double)i + 1.0);
    if (i & 1) {
      acc -= term;
    } else {
      acc += term;
    }
  }
  printf("%.12f\n", acc * 4.0);
  return 0;
}
