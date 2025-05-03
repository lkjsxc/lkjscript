#include <stdint.h>
#include <stdio.h>

#define SRC_PATH "./lkjscriptsrc"
#define MEM_SIZE (1024 * 1024 * 2)

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
    INST_CALL,
    INST_RETURN,
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
    vec_t* key;
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

bool_t vec_iseq(vec_t* vec1, vec_t* vec2) {
    if (vec1 == NULL || vec2 == NULL) {
        return FALSE;
    }
    if (vec1->size != vec2->size) {
        return FALSE;
    }
    for (int64_t i = 0; i < vec1->size; i++) {
        if (vec1->data[i] != vec2->data[i]) {
            return FALSE;
        }
    }
    return TRUE;
}

bool_t vec_iseqstr(vec_t* vec, const char* str) {
    int64_t str_size = 0;
    while (str[str_size] != '\0') {
        str_size++;
    }
    vec_t vec2 = (vec_t){.data = str, .size = str_size};
    return vec_iseq(vec, &vec2);
}

result_t compile_readsrc() {
    FILE* fp = fopen(SRC_PATH, "r");
    if (fp == NULL) {
        puts("Failed to open lkjscriptsrc");
        return ERR;
    }
    size_t n = fread(mem.compile.src, 1, sizeof(mem.compile.src) - 3, fp);
    mem.compile.src[n + 0] = '\n';
    mem.compile.src[n + 1] = '\0';
    mem.compile.src[n + 2] = '\0';
    fclose(fp);
    return OK;
}

result_t compile_tokenize() {
    vec_t* token_itr = mem.compile.token;
    const char* base_itr = mem.compile.src;
    const char* corrent_itr = mem.compile.src;
    while (1) {
        char ch1 = *(corrent_itr + 0);
        char ch2 = *(corrent_itr + 1);
        if (ch1 == '\0') {
            break;
        } else if (ch1 == '\n' || ch1 == ' ') {
            if (base_itr != corrent_itr) {
                *(token_itr++) = (vec_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            corrent_itr += 1;
            base_itr = corrent_itr;
        } else if (
            (ch1 == '<' && ch2 == '<') ||
            (ch1 == '>' && ch2 == '>') ||
            (ch1 == '<' && ch2 == '=') ||
            (ch1 == '>' && ch2 == '=') ||
            (ch1 == '=' && ch2 == '=') ||
            (ch1 == '!' && ch2 == '=') ||
            (ch1 == '&' && ch2 == '&') ||
            (ch1 == '|' && ch2 == '|')) {
            if (base_itr != corrent_itr) {
                *(token_itr++) = (vec_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            *(token_itr++) = (vec_t){.data = corrent_itr, .size = 2};
            corrent_itr += 2;
            base_itr = corrent_itr;
        } else if (ch1 == '(' || ch1 == ')' || ch1 == '{' || ch1 == '}' || ch1 == ';' || ch1 == ',' ||
                   ch1 == ':' || ch1 == '.' || ch1 == '+' || ch1 == '-' || ch1 == '*' || ch1 == '/' ||
                   ch1 == '%' || ch1 == '&' || ch1 == '|' || ch1 == '^' || ch1 == '~' || ch1 == '<' ||
                   ch1 == '>' || ch1 == '!' || ch1 == '=') {
            if (base_itr != corrent_itr) {
                *(token_itr++) = (vec_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            *(token_itr++) = (vec_t){.data = corrent_itr, .size = 1};
            corrent_itr += 1;
            base_itr = corrent_itr;
        } else {
            corrent_itr += 1;
        }
    }
    *(token_itr++) = (vec_t){.data = NULL, .size = 0};
    return OK;
}

result_t compile_stat(vec_t** token_itr, node_t** node_itr, int64_t* label_count, int64_t label_continue, int64_t label_break) {
    if (vec_iseqstr(*token_itr, "{")) {
        (*token_itr)++;
        while (vec_iseqstr(*token_itr, "}")) {
            if (compile_stat(token_itr, node_itr, label_count, label_continue, label_break) == ERR) {
                return ERR;
            }
        }
        (*token_itr)++;
    } else if (vec_iseqstr(*token_itr, "fn")) {
    } else if (vec_iseqstr(*token_itr, "if")) {
    } else if (vec_iseqstr(*token_itr, "loop")) {
    } else if (vec_iseqstr(*token_itr, "continue")) {
    } else if (vec_iseqstr(*token_itr, "break")) {
    } else if (vec_iseqstr(*token_itr, "return")) {
        (*token_itr)++;
        if (compile_expr(token_itr, node_itr, label_count, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = INST_RETURN, .token = NULL, .val = 0, .bin = NULL};
    } else {
        if (compile_expr(token_itr, node_itr, label_count, label_continue, label_break) == ERR) {
            return ERR;
        }
    }
    return OK;
}

result_t compile_parse() {
    vec_t* token_itr = mem.compile.token;
    node_t* node_itr = mem.compile.node;
    int64_t label_count = 0;
    while (token_itr->data != NULL) {
        if (compile_stat(&token_itr, &node_itr, &label_count, -1, -1) == ERR) {
            return ERR;
        }
    }
    return OK;
}

result_t execute() {
    return OK;
}

result_t compile() {
    if (compile_readsrc() == ERR) {
        puts("Failed to readsrc");
        return ERR;
    }
    if (compile_tokenize() == ERR) {
        puts("Failed to tokenize");
    }
    if (compile_parse() == ERR) {
        puts("Failed to parse");
        return ERR;
    }
    return OK;
}

int main() {
    if (compile() == ERR) {
        puts("Failed to compile");
        return 1;
    }
    if (execute() == ERR) {
        puts("Failed to execute");
    }
    puts(mem.compile.src);
    return 0;
}