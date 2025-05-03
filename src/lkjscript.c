#include <stdint.h>
#include <stdio.h>

#define SRC_PATH "./lkjscriptsrc"
#define MEM_SIZE (1024 * 1024 * 16)

typedef enum {
    FALSE = 0,
    TRUE = 1,
} bool_t;

typedef enum {
    OK = 0,
    ERR = 1,
} result_t;

typedef enum {
    GLOBALADDR_ZERO,
    GLOBALADDR_IP,
    GLOBALADDR_SP,
    GLOBALADDR_BP,
} globaladdr_t;

typedef enum {
    INST_NULL,
    INST_NOP,
} type_t;

typedef union {
    int64_t i64;
    type_t type;
} uni64_t;

typedef struct {
    const char* data;
    int64_t size;
} vec_t;

typedef struct {
    type_t type;
    const vec_t* token;
    int64_t val;
    uni64_t* bin;
} node_t;

typedef struct {
    vec_t key;
    int64_t val;
} pair_t;

typedef struct {
    uni64_t bin[MEM_SIZE / sizeof(uni64_t) / 6];
    char src[MEM_SIZE / sizeof(char) / 6];
    vec_t token[MEM_SIZE / sizeof(vec_t) / 6];
    node_t node[MEM_SIZE / sizeof(node_t) / 6];
    pair_t pair[MEM_SIZE / sizeof(pair_t) / 6];
} compile_t;

typedef union {
    uni64_t uni64[MEM_SIZE / sizeof(uni64_t)];
    compile_t compile;
} mem_t;

mem_t mem;

result_t compile_readsrc() {
    FILE* fp = fopen(SRC_PATH, "r");
    if (fp == NULL) {
        puts("Failed to open lkjscriptsrc");
        return ERR;
    }
    size_t n = fread(mem.compile.src, 1, sizeof(mem.compile.src) - 2, fp);
    mem.compile.src[n] = '\n';
    mem.compile.src[n + 1] = '\0';
    fclose(fp);
    return OK;
}

result_t compile() {
    if (compile_readsrc() != OK) {
        puts("Failed to compile_readsrc");
        return ERR;
    }
    return OK;
}

int main() {
    if (compile() != OK) {
        puts("Failed to compile");
        return 1;
    }
    puts(mem.compile.src);
    return 0;
}