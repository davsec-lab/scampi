#include <stdio.h>

int add(int a, int b) {
    return a + (b + 5);
}

int main() {
    int result = add(5, 7);
    printf("Result is %d", result);
    return 0;
}