#include <stdint.h>
#include <stdio.h>

#define SRC_PATH "./lkjscriptsrc"
#define MEM_SIZE (1024 * 1024 * 2)
#define MEM_GLOBAL_SIZE 1024

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

    TY_NULL,
    TY_INST_NOP,
    TY_INST_LABEL,
    TY_INST_PUSH_LOCAL_VAL,
    TY_INST_PUSH_LOCAL_ADDR,
    TY_INST_PUSH_CONST,
    TY_INST_JMP,
    TY_INST_JZ,
    TY_INST_CALL,
    TY_INST_RETURN,

    TY_INST_ASSIGN1,
    TY_INST_ASSIGN2,
    TY_INST_ASSIGN3,
    TY_INST_ASSIGN4,

    TY_INST_OR,
    TY_INST_AND,
    TY_INST_EQ,
    TY_INST_NE,
    TY_INST_LT,
    TY_INST_LE,
    TY_INST_GT,
    TY_INST_GE,
    TY_INST_NOT,
    TY_INST_ADD,
    TY_INST_SUB,
    TY_INST_MUL,
    TY_INST_DIV,
    TY_INST_MOD,
    TY_INST_SHL,
    TY_INST_SHR,
    TY_INST_BITOR,
    TY_INST_BITXOR,
    TY_INST_BITAND,

    TY_INST_DEREF,
    TY_INST_NEG,
    TY_INST_BITNOT,

    TY_LABEL,
    TY_LABEL_SCOPE_OPEN,
    TY_LABEL_SCOPE_CLOSE,
    TY_LABEL_STARTSCRIPT,

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
    pair_t map[MEM_SIZE / sizeof(pair_t) / 6];
} compile_t;

typedef union {
    uni64_t uni64[MEM_SIZE / sizeof(uni64_t)];
    compile_t compile;
} mem_t;

mem_t mem;

result_t compile_parse_expr(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break);

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

bool_t vec_isnum(vec_t* vec) {
    char ch = vec->data[0];
    return '0' <= ch && ch <= '9';
}

bool_t vec_isvar(vec_t* vec) {
    char ch = vec->data[0];
    return ('a' <= ch && ch <= 'z') || ('A' <= ch && ch <= 'Z') || ch == '_';
}

pair_t* map_find(vec_t* vec) {
    pair_t* itr;
    for (itr = mem.compile.map; itr->key != NULL; itr++) {
        if (vec_iseq(vec, itr->key)) {
            return itr;
        }
    }
    return itr;
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
    bool_t iscomment = FALSE;
    while (1) {
        char ch1 = *(corrent_itr + 0);
        char ch2 = *(corrent_itr + 1);
        if (ch1 == '\0') {
            break;
        } else if (ch1 == '\n') {
            if (!iscomment && base_itr != corrent_itr) {
                *(token_itr++) = (vec_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            iscomment = FALSE;
            *(token_itr++) = (vec_t){.data = corrent_itr, .size = 1};
            corrent_itr += 1;
            base_itr = corrent_itr;
        } else if (ch1 == '/' && ch2 == '/') {
            iscomment = TRUE;
            corrent_itr += 1;
        } else if (iscomment) {
            corrent_itr += 1;
        } else if (ch1 == ' ') {
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

result_t compile_parse_primary(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if ((*token_itr)->data == NULL) {
        puts("Error: Unexpected end of input in primary expression.");
        return ERR;
    } else if (vec_iseqstr(*token_itr, "(")) {
        if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse expression inside parentheses.");
            return ERR;
        }
    } else if (vec_isnum(*token_itr)) {
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = *token_itr, .val = 0, .bin = NULL};
        (*token_itr)++;
    } else if (vec_isvar(*token_itr)) {
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_VAL, .token = *token_itr, .val = 0, .bin = NULL};
        (*token_itr)++;
    } else {
        puts("Error: Unexpected token in primary expression.");
        return ERR;
    }
    return OK;
}

result_t compile_parse_postfix(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if ((map_find(*token_itr)->key != NULL) && vec_iseqstr(*token_itr + 1, "(")) {
        vec_t* fn_name = *token_itr;
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse function call arguments.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_CALL, .token = fn_name, .val = 0, .bin = NULL};
    } else {
        if (compile_parse_primary(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse primary expression in postfix.");
            return ERR;
        }
    }
    return OK;
}

result_t compile_parse_unary(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    while (1) {
        if (vec_iseqstr(*token_itr, "*")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse operand for dereference operator '*'.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_DEREF, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "+")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse operand for unary plus operator '+'.");
                return ERR;
            }
        } else if (vec_iseqstr(*token_itr, "-")) {
            (*token_itr)++;
            *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = NULL, .val = 0, .bin = NULL};
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse operand for unary minus operator '-'.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SUB, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "~")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse operand for bitwise not operator '~'.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_BITNOT, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "!")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse operand for logical not operator '!'.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_NOT, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "&")) {
            (*token_itr)++;

            if (!vec_isvar(*token_itr)) {
                puts("Error: Expected variable after address-of operator '&'.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_ADDR, .token = *token_itr, .val = 0, .bin = NULL};
            (*token_itr)++;
            return OK;
        } else {
            if (compile_parse_postfix(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse postfix expression in unary.");
                return ERR;
            }
            return OK;
        }
    }
}

result_t compile_parse_mul(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_unary(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for multiplicative expression.");
        return ERR;
    }
    while (1) {
        if (vec_iseqstr(*token_itr, "*")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '*' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_MUL, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "/")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '/' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_DIV, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "%")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '%' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_MOD, .token = NULL, .val = 0, .bin = NULL};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_add(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_mul(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for additive expression.");
        return ERR;
    }
    while (1) {
        if (vec_iseqstr(*token_itr, "+")) {
            (*token_itr)++;
            if (compile_parse_mul(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '+' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_ADD, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "-")) {
            (*token_itr)++;
            if (compile_parse_mul(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '-' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SUB, .token = NULL, .val = 0, .bin = NULL};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_shift(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_add(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for shift expression.");
        return ERR;
    }
    while (1) {
        if (vec_iseqstr(*token_itr, "<<")) {
            (*token_itr)++;
            if (compile_parse_add(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '<<' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SHL, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, ">>")) {
            (*token_itr)++;
            if (compile_parse_add(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '>>' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SHR, .token = NULL, .val = 0, .bin = NULL};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_rel(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_shift(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for relational expression.");
        return ERR;
    }
    while (1) {
        if (vec_iseqstr(*token_itr, "<")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '<' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_LT, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, ">")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '>' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_GT, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "<=")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '<=' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_LE, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, ">=")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '>=' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_GE, .token = NULL, .val = 0, .bin = NULL};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_eq(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_rel(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for equality expression.");
        return ERR;
    }
    while (1) {
        if (vec_iseqstr(*token_itr, "==")) {
            (*token_itr)++;
            if (compile_parse_rel(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '==' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_EQ, .token = NULL, .val = 0, .bin = NULL};
        } else if (vec_iseqstr(*token_itr, "!=")) {
            (*token_itr)++;
            if (compile_parse_rel(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse right operand for '!=' operator.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_NE, .token = NULL, .val = 0, .bin = NULL};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_bit_and(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_eq(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for bitwise AND expression.");
        return ERR;
    }
    while (vec_iseqstr(*token_itr, "&")) {
        (*token_itr)++;
        if (compile_parse_eq(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '&' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITAND, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_bit_xor(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_and(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for bitwise XOR expression.");
        return ERR;
    }
    while (vec_iseqstr(*token_itr, "^")) {
        (*token_itr)++;
        if (compile_parse_bit_and(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '^' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITXOR, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_bit_or(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_xor(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for bitwise OR expression.");
        return ERR;
    }
    while (vec_iseqstr(*token_itr, "|")) {
        (*token_itr)++;
        if (compile_parse_bit_xor(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '|' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITOR, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_and(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_or(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for logical AND expression.");
        return ERR;
    }
    while (vec_iseqstr(*token_itr, "&&")) {
        (*token_itr)++;
        if (compile_parse_bit_or(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '&&' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_AND, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_or(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_and(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for logical OR expression.");
        return ERR;
    }
    while (vec_iseqstr(*token_itr, "||")) {
        (*token_itr)++;
        if (compile_parse_and(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '||' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_OR, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_assign(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_or(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
        puts("Error: Failed to parse left operand for assignment.");
        return ERR;
    }
    if (vec_iseqstr(*token_itr, "=")) {
        node_t* lval_node = (*node_itr) - 1;
        if (lval_node < mem.compile.node ||
            (lval_node->type != TY_INST_PUSH_LOCAL_ADDR && lval_node->type != TY_INST_DEREF)) {
            puts("Error: Invalid left-hand side in assignment.");
            return ERR;
        }

        (*token_itr)++;
        if (compile_parse_or(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse right operand for '=' operator.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_ASSIGN1, .token = NULL, .val = 0, .bin = NULL};
    }
    return OK;
}

result_t compile_parse_expr(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (vec_iseqstr(*token_itr, "(")) {
        (*token_itr)++;
        while (!vec_iseqstr(*token_itr, ")")) {
            if ((*token_itr)->data == NULL) {
                puts("Error: Unterminated parenthesized expression or argument list.");
                return ERR;
            }
            if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse expression within parentheses or argument list.");
                return ERR;
            }
            while (vec_iseqstr(*token_itr, ",") || vec_iseqstr(*token_itr, "\n")) {
                (*token_itr)++;
            }
        }
        (*token_itr)++;
    } else {
        if (compile_parse_assign(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse assignment or lower precedence expression.");
            return ERR;
        }
    }
    return OK;
}

result_t compile_parse_stat(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    if (vec_iseqstr(*token_itr, "{")) {
        (*token_itr)++;
        while (vec_iseqstr(*token_itr, "\n")) {
            (*token_itr)++;
        }
        while (!vec_iseqstr(*token_itr, "}")) {
            if ((*token_itr)->data == NULL) {
                puts("Error: Unterminated block statement.");
                return ERR;
            }
            if (compile_parse_stat(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse statement within block.");
                return ERR;
            }
            while (vec_iseqstr(*token_itr, "\n")) {
                (*token_itr)++;
            }
        }
        (*token_itr)++;
    } else if (vec_iseqstr(*token_itr, "if")) {
        int64_t label_if = (*label_cnt)++;
        int64_t label_else = (*label_cnt)++;
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse condition for 'if' statement.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_JZ, .token = NULL, .val = label_if, .bin = NULL};
        if (compile_parse_stat(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse body for 'if' statement.");
            return ERR;
        }
        if (vec_iseqstr(*token_itr, "else")) {
            (*token_itr)++;
            *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_else, .bin = NULL};
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_if, .bin = NULL};
            if (compile_parse_stat(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
                puts("Error: Failed to parse body for 'else' statement.");
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_else, .bin = NULL};
        } else {
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_if, .bin = NULL};
        }
    } else if (vec_iseqstr(*token_itr, "loop")) {
        int64_t label_start = (*label_cnt)++;
        int64_t label_end = (*label_cnt)++;
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_start, .bin = NULL};
        if (compile_parse_stat(token_itr, node_itr, label_cnt, label_start, label_end) == ERR) {
            puts("Error: Failed to parse body for 'loop' statement.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_start, .bin = NULL};
        *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_end, .bin = NULL};
    } else if (vec_iseqstr(*token_itr, "continue")) {
        if (label_continue == -1) {
            puts("Error: 'continue' statement outside of loop.");
            return ERR;
        }
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_continue, .bin = NULL};
    } else if (vec_iseqstr(*token_itr, "break")) {
        if (label_break == -1) {
            puts("Error: 'break' statement outside of loop.");
            return ERR;
        }
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_break, .bin = NULL};
    } else if (vec_iseqstr(*token_itr, "return")) {
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse return value expression.");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_RETURN, .token = NULL, .val = 0, .bin = NULL};
    } else {
        if (compile_parse_expr(token_itr, node_itr, label_cnt, label_continue, label_break) == ERR) {
            puts("Error: Failed to parse expression statement.");
            return ERR;
        }

        if (vec_iseqstr(*token_itr, ";")) {
            (*token_itr)++;
        }
    }
    return OK;
}

result_t compile_parse_fn(vec_t** token_itr, node_t** node_itr, int64_t* label_cnt, int64_t label_continue, int64_t label_break) {
    int64_t label_open = (*label_cnt)++;
    int64_t label_close = (*label_cnt)++;
    int64_t arg_cnt = 0;

    if (!vec_isvar(*token_itr + 1) || !vec_iseqstr(*token_itr + 2, "(")) {
        puts("Error: Invalid function definition syntax. Expected 'fn name(...)'.");
        return ERR;
    }
    vec_t* fn_name = *token_itr + 1;
    pair_t* fn_map = map_find(fn_name);

    if (fn_map->key == NULL) {
        puts("Error: Function name not found in pre-scan map during definition parsing.");
        return ERR;
    }
    fn_map->val = label_open;

    (*token_itr) += 3;

    node_t* arg_list_start = *node_itr;
    while (!vec_iseqstr(*token_itr, ")")) {
        if ((*token_itr)->data == NULL) {
            puts("Error: Unexpected end of input while parsing function arguments.");
            return ERR;
        }
        if (!vec_isvar(*token_itr)) {
            puts("Error: Expected identifier for function argument name.");
            return ERR;
        }

        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_ADDR, .token = *token_itr, .val = 0 /* Placeholder */, .bin = NULL};
        arg_cnt++;
        (*token_itr)++;
        if (vec_iseqstr(*token_itr, ",")) {
            (*token_itr)++;
            while (vec_iseqstr(*token_itr, "\n")) {
                (*token_itr)++;
            }
        } else if (!vec_iseqstr(*token_itr, ")")) {
            puts("Error: Expected ',' or ')' after function argument.");
            return ERR;
        }
        while (vec_iseqstr(*token_itr, "\n")) {
            (*token_itr)++;
        }
    }
    (*token_itr)++;

    node_t* arg_node_itr = arg_list_start;
    for (int64_t i = 0; i < arg_cnt; i++) {
        arg_node_itr->val = -i - 3;
        arg_node_itr++;
    }

    *((*node_itr)++) = (node_t){.type = TY_LABEL_SCOPE_OPEN, .token = NULL, .val = label_open, .bin = NULL};
    *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = fn_name, .val = label_open /* Store label again? */, .bin = NULL};

    if (compile_parse_stat(token_itr, node_itr, label_cnt, -1, -1) == ERR) {
        puts("Error: Failed to parse function body.");
        return ERR;
    }

    *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = NULL, .val = 0, .bin = NULL};
    *((*node_itr)++) = (node_t){.type = TY_INST_RETURN, .token = NULL, .val = 0, .bin = NULL};
    *((*node_itr)++) = (node_t){.type = TY_LABEL_SCOPE_CLOSE, .token = NULL, .val = label_close, .bin = NULL};
    return OK;
}

result_t compile_parse() {
    vec_t* token_itr = mem.compile.token;
    node_t* node_itr = mem.compile.node;
    pair_t* map_itr = mem.compile.map;
    int64_t label_cnt = 0;

    vec_t* scan_itr = mem.compile.token;
    while (scan_itr->data != NULL) {
        if (vec_iseqstr(scan_itr, "fn")) {
            if (scan_itr[1].data != NULL && vec_isvar(&scan_itr[1])) {
                pair_t* existing = map_find(&scan_itr[1]);
                if (existing->key != NULL) {
                    puts("Error: Duplicate function definition.");
                    return ERR;
                }
                *(map_itr++) = (pair_t){.key = &scan_itr[1], .val = 0};
            } else {
                puts("Error: Expected identifier after 'fn'.");
                return ERR;
            }
        }

        while (scan_itr->data != NULL && !vec_iseqstr(scan_itr, "\n"))
            scan_itr++;
        if (scan_itr->data != NULL)
            scan_itr++;
    }
    *(map_itr) = (pair_t){.key = NULL, .val = 0};

    token_itr = mem.compile.token;
    node_itr = mem.compile.node;
    map_itr = mem.compile.map;

    while (vec_iseqstr(token_itr, "\n")) {
        token_itr++;
    }
    while (vec_iseqstr(token_itr, "fn")) {
        if (compile_parse_fn(&token_itr, &node_itr, &label_cnt, -1, -1) == ERR) {
            puts("Error: Failed during function definition parsing.");
            return ERR;
        }
        while (vec_iseqstr(token_itr, "\n")) {
            token_itr++;
        }
    }
    *(node_itr++) = (node_t){.type = TY_LABEL_STARTSCRIPT, .token = NULL, .val = 0, .bin = NULL};
    while (vec_iseqstr(token_itr, "\n")) {
        token_itr++;
    }
    while (token_itr->data != NULL) {
        if (compile_parse_stat(&token_itr, &node_itr, &label_cnt, -1, -1) == ERR) {
            puts("Error: Failed to parse after define function.");
            return ERR;
        }
        while (vec_iseqstr(token_itr, "\n")) {
            token_itr++;
        }
    }
    *node_itr = (node_t){.type = TY_NULL, .token = NULL, .val = 0, .bin = NULL};
    return OK;
}

result_t compile_bingen() {
    uni64_t* bin_begin = mem.compile.bin + (MEM_GLOBAL_SIZE / sizeof(uni64_t));
    uni64_t* bin_itr = bin_begin;
    node_t* node_itr = mem.compile.node;
    int64_t localval_offset = 0;
    pair_t* map_itr = mem.compile.map;
    while (map_itr->key != NULL) {
        map_itr++;
    }

    return OK;
}

result_t execute() {
    return OK;
}

result_t compile() {
    if (compile_readsrc() == ERR) {
        return ERR;
    }
    if (compile_tokenize() == ERR) {
        puts("Failed to tokenize");
        return ERR;
    }
    if (compile_parse() == ERR) {
        return ERR;
    }
    if (compile_bingen() == ERR) {
        puts("Failed to bingen");
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
        return 1;
    }

    puts("Execution finished successfully.");
    return 0;
}