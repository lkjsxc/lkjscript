#include <stdio.h>

#define SRC_PATH "./lkjscriptsrc"
#define MEM_SIZE 1024

char mem[MEM_SIZE];

int main() {
    FILE* fp = fopen(SRC_PATH, "r");
    if(fp == NULL) {
        puts("Failed to open lkjscriptsrc.");
        return -1;
    }
    size_t n = fread(mem, 1, sizeof(mem), fp);
    fclose(fp);
    mem[n] = '\0';
    puts(mem);
    return 0;
}