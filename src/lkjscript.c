#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

#define SRC_PATH "./lkjscriptsrc"
#define MEM_SIZE (1024 * 256)
#define MEM_GLOBAL_SIZE 32
#define MEM_STACK_SIZE 256

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
    TY_INST_PUSH_CONST,
    TY_INST_PUSH_LOCAL_VAL,
    TY_INST_PUSH_LOCAL_ADDR,
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

    TY_INST_READ,
    TY_INST_WRITE,

    TY_LABEL,
    TY_LABEL_SCOPE_OPEN,
    TY_LABEL_SCOPE_CLOSE,
    TY_LABEL_STARTSCRIPT,

} type_t;

typedef struct {
    const char* data;
    int64_t size;
} token_t;

typedef struct {
    type_t type;
    token_t* token;
    int64_t val;
} node_t;

typedef struct {
    token_t* key;
    int64_t val;
} pair_t;

typedef struct {
    int64_t bin[MEM_SIZE / sizeof(int64_t) / 6];
    char src[MEM_SIZE / sizeof(char) / 6];
    token_t token[MEM_SIZE / sizeof(token_t) / 6];
    node_t node[MEM_SIZE / sizeof(node_t) / 6];
    pair_t map[MEM_SIZE / sizeof(pair_t) / 6];
} compile_t;

typedef union {
    int64_t bin[MEM_SIZE / sizeof(int64_t)];
    compile_t compile;
} mem_t;

mem_t mem;

result_t compile_parse_expr(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break);

bool_t token_iseq(token_t* token1, token_t* token2) {
    if (token1 == NULL || token2 == NULL) {
        return FALSE;
    }
    if (token1->size != token2->size) {
        return FALSE;
    }
    for (int64_t i = 0; i < token1->size; i++) {
        if (token1->data[i] != token2->data[i]) {
            return FALSE;
        }
    }
    return TRUE;
}

bool_t token_iseqstr(token_t* token, const char* str) {
    int64_t str_size = 0;
    while (str[str_size] != '\0') {
        str_size++;
    }
    token_t token2 = (token_t){.data = str, .size = str_size};
    return token_iseq(token, &token2);
}

bool_t token_isnum(token_t* token) {
    char ch = token->data[0];
    return '0' <= ch && ch <= '9';
}

bool_t token_isvar(token_t* token) {
    char ch = token->data[0];
    return ('a' <= ch && ch <= 'z') || ('A' <= ch && ch <= 'Z') || ch == '_';
}

int64_t token_toint(token_t* token) {
    int64_t result = 0;
    int64_t sign = 1;
    int64_t i = 0;
    if (token->data[0] == '-') {
        sign = -1;
        i = 1;
    }
    for (; i < token->size; i++) {
        result = result * 10 + (token->data[i] - '0');
    }
    return result * sign;
}

pair_t* map_find(token_t* token, int64_t map_cnt) {
    for (int64_t i = 0; i < map_cnt; i++) {
        if (token_iseq(token, mem.compile.map[i].key)) {
            return &mem.compile.map[i];
        }
    }
    return &mem.compile.map[map_cnt];
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
    token_t* token_itr = mem.compile.token;
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
                *(token_itr++) = (token_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            iscomment = FALSE;
            *(token_itr++) = (token_t){.data = corrent_itr, .size = 1};
            corrent_itr += 1;
            base_itr = corrent_itr;
        } else if (ch1 == '/' && ch2 == '/') {
            iscomment = TRUE;
            corrent_itr += 1;
        } else if (iscomment) {
            corrent_itr += 1;
        } else if (ch1 == ' ') {
            if (base_itr != corrent_itr) {
                *(token_itr++) = (token_t){.data = base_itr, .size = corrent_itr - base_itr};
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
                *(token_itr++) = (token_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            *(token_itr++) = (token_t){.data = corrent_itr, .size = 2};
            corrent_itr += 2;
            base_itr = corrent_itr;
        } else if (ch1 == '(' || ch1 == ')' || ch1 == '{' || ch1 == '}' || ch1 == ';' || ch1 == ',' ||
                   ch1 == ':' || ch1 == '.' || ch1 == '+' || ch1 == '-' || ch1 == '*' || ch1 == '/' ||
                   ch1 == '%' || ch1 == '&' || ch1 == '|' || ch1 == '^' || ch1 == '~' || ch1 == '<' ||
                   ch1 == '>' || ch1 == '!' || ch1 == '=') {
            if (base_itr != corrent_itr) {
                *(token_itr++) = (token_t){.data = base_itr, .size = corrent_itr - base_itr};
            }
            *(token_itr++) = (token_t){.data = corrent_itr, .size = 1};
            corrent_itr += 1;
            base_itr = corrent_itr;
        } else {
            corrent_itr += 1;
        }
    }
    *(token_itr++) = (token_t){.data = NULL, .size = 0};
    return OK;
}

void compile_parse_skiplinebreak(token_t** token_itr) {
    while (token_iseqstr(*token_itr, "\n")) {
        (*token_itr)++;
    }
}

result_t compile_parse_primary(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if ((*token_itr)->data == NULL) {
        return ERR;
    } else if (token_iseqstr(*token_itr, "(")) {
        if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
    } else if (token_isnum(*token_itr)) {
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = *token_itr, .val = token_toint(*token_itr)};
        (*token_itr)++;
    } else if (token_isvar(*token_itr)) {
        if (token_iseqstr(*token_itr, "if")) {
            puts("error: primary observed if");
            return ERR;
        } else if (token_iseqstr(*token_itr, "else")) {
            puts("error: primary observed else");
            return ERR;
        } else if (token_iseqstr(*token_itr, "loop")) {
            puts("error: primary observed loop");
            return ERR;
        } else if (token_iseqstr(*token_itr, "continue")) {
            puts("error: primary observed continue");
            return ERR;
        } else if (token_iseqstr(*token_itr, "break")) {
            puts("error: primary observed break");
            return ERR;
        } else if (token_iseqstr(*token_itr, "fn")) {
            puts("error: primary observed fn");
            return ERR;
        } else if (token_iseqstr(*token_itr, "return")) {
            puts("error: primary observed return");
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_VAL, .token = *token_itr, .val = 0};
        (*token_itr)++;
    } else {
        return ERR;
    }
    return OK;
}

result_t compile_parse_postfix(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if ((map_find(*token_itr, *map_cnt)->key != NULL) && token_iseqstr(*token_itr + 1, "(")) {
        token_t* fn_name = *token_itr;
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_CALL, .token = fn_name, .val = 0};
    } else {
        if (compile_parse_primary(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
    }
    return OK;
}

result_t compile_parse_unary(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    while (1) {
        if (token_iseqstr(*token_itr, "*")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_DEREF, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "+")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
        } else if (token_iseqstr(*token_itr, "-")) {
            (*token_itr)++;
            *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = NULL, .val = 0};
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SUB, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "~")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_BITNOT, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "!")) {
            (*token_itr)++;
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_NOT, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "&")) {
            (*token_itr)++;
            *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_ADDR, .token = *token_itr, .val = 0};
            return OK;
        } else {
            if (compile_parse_postfix(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            return OK;
        }
    }
}

result_t compile_parse_mul(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_unary(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (1) {
        if (token_iseqstr(*token_itr, "*")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_MUL, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "/")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_DIV, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "%")) {
            (*token_itr)++;
            if (compile_parse_unary(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_MOD, .token = NULL, .val = 0};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_add(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_mul(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (1) {
        if (token_iseqstr(*token_itr, "+")) {
            (*token_itr)++;
            if (compile_parse_mul(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_ADD, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "-")) {
            (*token_itr)++;
            if (compile_parse_mul(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SUB, .token = NULL, .val = 0};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_shift(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_add(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (1) {
        if (token_iseqstr(*token_itr, "<<")) {
            (*token_itr)++;
            if (compile_parse_add(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SHL, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, ">>")) {
            (*token_itr)++;
            if (compile_parse_add(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_SHR, .token = NULL, .val = 0};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_rel(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_shift(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (1) {
        if (token_iseqstr(*token_itr, "<")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_LT, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, ">")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_GT, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "<=")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_LE, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, ">=")) {
            (*token_itr)++;
            if (compile_parse_shift(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_GE, .token = NULL, .val = 0};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_eq(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_rel(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (1) {
        if (token_iseqstr(*token_itr, "==")) {
            (*token_itr)++;
            if (compile_parse_rel(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_EQ, .token = NULL, .val = 0};
        } else if (token_iseqstr(*token_itr, "!=")) {
            (*token_itr)++;
            if (compile_parse_rel(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_INST_NE, .token = NULL, .val = 0};
        } else {
            return OK;
        }
    }
}

result_t compile_parse_bit_and(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_eq(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (token_iseqstr(*token_itr, "&")) {
        (*token_itr)++;
        if (compile_parse_eq(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITAND, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_bit_xor(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_and(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (token_iseqstr(*token_itr, "^")) {
        (*token_itr)++;
        if (compile_parse_bit_and(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITXOR, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_bit_or(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_xor(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (token_iseqstr(*token_itr, "|")) {
        (*token_itr)++;
        if (compile_parse_bit_xor(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_BITOR, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_and(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_bit_or(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (token_iseqstr(*token_itr, "&&")) {
        (*token_itr)++;
        if (compile_parse_bit_or(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_AND, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_or(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_and(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    while (token_iseqstr(*token_itr, "||")) {
        (*token_itr)++;
        if (compile_parse_and(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_OR, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_assign(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    if (compile_parse_or(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    if (token_iseqstr(*token_itr, "=")) {
        (*token_itr)++;
        if (compile_parse_or(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_ASSIGN1, .token = NULL, .val = 0};
    }
    return OK;
}

result_t compile_parse_expr(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    while (token_iseqstr(*token_itr, ",")) {
        (*token_itr)++;
    }
    if (token_iseqstr(*token_itr, "(")) {
        (*token_itr)++;
        while (!token_iseqstr(*token_itr, ")")) {
            if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            if (token_iseqstr(*token_itr, ",")) {
                (*token_itr)++;
            }
        }
        (*token_itr)++;
    } else {
        if (compile_parse_assign(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
    }
    return OK;
}

result_t compile_parse_stat(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    compile_parse_skiplinebreak(token_itr);
    if (token_iseqstr(*token_itr, "{")) {
        (*token_itr)++;
        compile_parse_skiplinebreak(token_itr);
        while (!token_iseqstr(*token_itr, "}")) {
            if (compile_parse_stat(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
        }
        (*token_itr)++;
    } else if (token_iseqstr(*token_itr, "if")) {
        int64_t label_if = (*map_cnt)++;
        int64_t label_else = (*map_cnt)++;
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_JZ, .token = NULL, .val = label_if};
        if (compile_parse_stat(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        if (token_iseqstr(*token_itr, "else")) {
            (*token_itr)++;
            *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_else};
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_if};
            if (compile_parse_stat(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
                return ERR;
            }
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_else};
        } else {
            *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_if};
        }
    } else if (token_iseqstr(*token_itr, "loop")) {
        int64_t label_start = (*map_cnt)++;
        int64_t label_end = (*map_cnt)++;
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_start};
        if (compile_parse_stat(token_itr, node_itr, map_cnt, label_start, label_end) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_start};
        *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = NULL, .val = label_end};
    } else if (token_iseqstr(*token_itr, "continue")) {
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_continue};
    } else if (token_iseqstr(*token_itr, "break")) {
        (*token_itr)++;
        *((*node_itr)++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = label_break};
    } else if (token_iseqstr(*token_itr, "return")) {
        (*token_itr)++;
        if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_RETURN, .token = NULL, .val = 0};
    } else {
        if (compile_parse_expr(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
            return ERR;
        }
    }
    compile_parse_skiplinebreak(token_itr);
    return OK;
}

result_t compile_parse_fn(token_t** token_itr, node_t** node_itr, int64_t* map_cnt, int64_t label_continue, int64_t label_break) {
    token_t* fn_name = *token_itr + 1;
    pair_t* fn_map = map_find(fn_name, *map_cnt);
    int64_t arg_cnt = 0;

    if (fn_map->key == NULL) {
        return ERR;
    }

    (*token_itr) += 3;
    while (!token_iseqstr(*token_itr, ")")) {
        if ((*token_itr)->data == NULL) {
            return ERR;
        }
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_ADDR, .token = *token_itr, .val = 0};
        (*token_itr)++;
        arg_cnt++;
        if (token_iseqstr(*token_itr, ",")) {
            (*token_itr)++;
        }
    }
    (*token_itr)++;

    node_t* arg_itr = *node_itr - 1;
    for (int64_t i = 0; i < arg_cnt; i++) {
        arg_itr->val = -i - 4;
        arg_itr--;
    }

    *((*node_itr)++) = (node_t){.type = TY_LABEL_SCOPE_OPEN, .token = NULL, .val = 0};
    *((*node_itr)++) = (node_t){.type = TY_LABEL, .token = fn_name, .val = fn_map - mem.compile.map};
    if (arg_cnt > 0) {
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_ADDR, .token = NULL, .val = -2};
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_LOCAL_VAL, .token = NULL, .val = -2};
        *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = NULL, .val = arg_cnt};
        *((*node_itr)++) = (node_t){.type = TY_INST_SUB, .token = NULL, .val = 0};
        *((*node_itr)++) = (node_t){.type = TY_INST_ASSIGN1, .token = NULL, .val = 0};
    }
    if (compile_parse_stat(token_itr, node_itr, map_cnt, label_continue, label_break) == ERR) {
        return ERR;
    }
    *((*node_itr)++) = (node_t){.type = TY_INST_PUSH_CONST, .token = NULL, .val = 0};
    *((*node_itr)++) = (node_t){.type = TY_INST_RETURN, .token = NULL, .val = 0};
    *((*node_itr)++) = (node_t){.type = TY_LABEL_SCOPE_CLOSE, .token = NULL, .val = 0};
    return OK;
}

result_t compile_parse(int64_t* map_cnt) {
    int64_t firstjmp = (*map_cnt)++;
    token_t* token_itr = mem.compile.token;
    node_t* node_itr = mem.compile.node;
    *(node_itr++) = (node_t){.type = TY_INST_JMP, .token = NULL, .val = firstjmp};
    while (token_itr->data != NULL) {
        if (token_iseqstr(token_itr, "fn")) {
            mem.compile.map[(*map_cnt)++] = (pair_t){.key = token_itr + 1, .val = 0};
        }
        token_itr++;
    }
    token_itr = mem.compile.token;
    compile_parse_skiplinebreak(&token_itr);
    while (token_iseqstr(token_itr, "fn")) {
        if (compile_parse_fn(&token_itr, &node_itr, map_cnt, -1, -1) == ERR) {
            return ERR;
        }
    }
    *(node_itr++) = (node_t){.type = TY_LABEL, .token = NULL, .val = firstjmp};
    while (token_itr->data != NULL) {
        if (compile_parse_stat(&token_itr, &node_itr, map_cnt, -1, -1) == ERR) {
            return ERR;
        }
    }
    *node_itr = (node_t){.type = TY_NULL, .token = NULL, .val = 0};
    return OK;
}

result_t compile_analyze(int64_t* map_cnt) {
    int64_t map_base = *map_cnt;
    node_t* node_itr = mem.compile.node;
    int64_t offset = 0;

    while (node_itr->type != TY_NULL) {
        if ((node_itr->type == TY_INST_PUSH_LOCAL_VAL || node_itr->type == TY_INST_PUSH_LOCAL_ADDR) && node_itr->token != NULL) {
            pair_t* map_result = map_find(node_itr->token, *map_cnt);
            if (map_result->key == NULL) {
                if (node_itr->val != 0) {
                    mem.compile.map[(*map_cnt)++] = (pair_t){.key = node_itr->token, .val = node_itr->val};
                } else {
                    mem.compile.map[(*map_cnt)++] = (pair_t){.key = node_itr->token, .val = offset++};
                }
            }
            node_itr->val = map_result->val;
        } else if (node_itr->type == TY_INST_CALL) {
            node_itr->val = map_find(node_itr->token, *map_cnt) - mem.compile.map;
        } else if (node_itr->type == TY_LABEL_SCOPE_CLOSE) {
            (*map_cnt) = map_base;
            offset = 0;
        }
        node_itr++;
    }
    return OK;
}

result_t compile_tobin() {
    int64_t* bin_base = mem.compile.bin + MEM_GLOBAL_SIZE;
    node_t* node_itr = mem.compile.node;
    int64_t* bin_itr = bin_base;
    while (node_itr->type != TY_NULL) {
        if (node_itr->type == TY_LABEL) {
            mem.compile.map[node_itr->val].val = bin_itr - mem.compile.bin;
        } else if (node_itr->type == TY_INST_PUSH_CONST || node_itr->type == TY_INST_PUSH_LOCAL_VAL || node_itr->type == TY_INST_PUSH_LOCAL_ADDR) {
            *(bin_itr++) = node_itr->type;
            *(bin_itr++) = node_itr->val;
        } else if (node_itr->type == TY_INST_JMP || node_itr->type == TY_INST_JZ || node_itr->type == TY_INST_CALL) {
            *(bin_itr++) = node_itr->type;
            *(bin_itr++) = node_itr->val;
        } else {
            if (node_itr->type != TY_LABEL_SCOPE_OPEN && node_itr->type != TY_LABEL_SCOPE_CLOSE && node_itr->type != TY_LABEL_STARTSCRIPT) {
                *(bin_itr++) = node_itr->type;
            }
        }
        node_itr++;
    }
    mem.bin[GLOBALADDR_IP] = MEM_GLOBAL_SIZE;
    mem.bin[GLOBALADDR_BP] = bin_itr - mem.compile.bin;
    mem.bin[GLOBALADDR_SP] = mem.bin[GLOBALADDR_BP] + MEM_STACK_SIZE;
    return OK;
}

result_t compile_link() {
    int64_t* bin_base = mem.compile.bin + MEM_GLOBAL_SIZE;
    int64_t* bin_itr = bin_base;
    while (*bin_itr != TY_NULL) {
        if (*bin_itr == TY_INST_PUSH_CONST || *bin_itr == TY_INST_PUSH_LOCAL_VAL || *bin_itr == TY_INST_PUSH_LOCAL_ADDR) {
            bin_itr += 2;
        } else if (*bin_itr == TY_INST_JMP || *bin_itr == TY_INST_JZ || *bin_itr == TY_INST_CALL) {
            *(bin_itr + 1) = mem.compile.map[*(bin_itr + 1)].val;
            bin_itr += 2;
        } else {
            bin_itr += 1;
        }
    }
    return OK;
}

result_t compile() {
    int64_t map_cnt = 0;
    if (compile_readsrc() == ERR) {
        puts("Failed to readsrc");
        return ERR;
    }
    if (compile_tokenize() == ERR) {
        puts("Failed to tokenize");
        return ERR;
    }
    if (compile_parse(&map_cnt) == ERR) {
        puts("Failed to parse");
        return ERR;
    }
    if (compile_analyze(&map_cnt) == ERR) {
        puts("Failed to analyze");
        return ERR;
    }
    if (compile_tobin() == ERR) {
        puts("Failed to tobin");
        return ERR;
    }
    if (compile_link() == ERR) {
        puts("Failed to link");
        return ERR;
    }
    return OK;
}

result_t execute() {
    while (TRUE) {
        switch (mem.bin[mem.bin[GLOBALADDR_IP]++]) {
            case TY_INST_PUSH_LOCAL_VAL: {
                int32_t addr = mem.bin[mem.bin[GLOBALADDR_IP]++] + mem.bin[GLOBALADDR_BP];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = mem.bin[addr];
            } break;
            case TY_INST_PUSH_LOCAL_ADDR: {
                int32_t addr = mem.bin[mem.bin[GLOBALADDR_IP]++] + mem.bin[GLOBALADDR_BP];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = addr;
            } break;
            case TY_INST_PUSH_CONST: {
                int32_t val = mem.bin[mem.bin[GLOBALADDR_IP]++];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val;
            } break;
            case TY_INST_DEREF: {
                int32_t addr = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = mem.bin[addr];
            } break;
            case TY_INST_ASSIGN1:
            case TY_INST_ASSIGN2:
            case TY_INST_ASSIGN3:
            case TY_INST_ASSIGN4: {
                int32_t val = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t addr = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[addr] = val;
            } break;
            case TY_INST_CALL: {
                mem.bin[mem.bin[GLOBALADDR_SP] + 0] = mem.bin[GLOBALADDR_IP] + 1;
                mem.bin[mem.bin[GLOBALADDR_SP] + 1] = mem.bin[GLOBALADDR_SP];
                mem.bin[mem.bin[GLOBALADDR_SP] + 2] = mem.bin[GLOBALADDR_BP];
                mem.bin[GLOBALADDR_IP] = mem.bin[mem.bin[GLOBALADDR_IP]];
                mem.bin[GLOBALADDR_BP] = mem.bin[GLOBALADDR_SP] + 3;
                mem.bin[GLOBALADDR_SP] += MEM_STACK_SIZE;
            } break;
            case TY_INST_RETURN: {
                int32_t ret_val = mem.bin[mem.bin[GLOBALADDR_SP] - 1];
                mem.bin[GLOBALADDR_IP] = mem.bin[mem.bin[GLOBALADDR_BP] - 3];
                mem.bin[GLOBALADDR_SP] = mem.bin[mem.bin[GLOBALADDR_BP] - 2];
                mem.bin[GLOBALADDR_BP] = mem.bin[mem.bin[GLOBALADDR_BP] - 1];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = ret_val;
            } break;
            case TY_INST_JMP: {
                int32_t addr = mem.bin[mem.bin[GLOBALADDR_IP]++];
                mem.bin[GLOBALADDR_IP] = addr;
            } break;
            case TY_INST_JZ: {
                int32_t addr = mem.bin[mem.bin[GLOBALADDR_IP]++];
                int32_t val = mem.bin[--mem.bin[GLOBALADDR_SP]];
                if (val == 0) {
                    mem.bin[GLOBALADDR_IP] = addr;
                }
            } break;
            case TY_INST_OR: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 | val2;
            } break;
            case TY_INST_AND: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 & val2;
            } break;
            case TY_INST_EQ: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 == val2;
            } break;
            case TY_INST_NE: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 != val2;
            } break;
            case TY_INST_LT: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 < val2;
            } break;
            case TY_INST_LE: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 <= val2;
            } break;
            case TY_INST_GT: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 > val2;
            } break;
            case TY_INST_GE: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 >= val2;
            } break;
            case TY_INST_ADD: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 + val2;
            } break;
            case TY_INST_SUB: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 - val2;
            } break;
            case TY_INST_MUL: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 * val2;
            } break;
            case TY_INST_DIV: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                if (val2 == 0) {
                    mem.bin[mem.bin[GLOBALADDR_SP]++] = INT64_MAX;
                } else {
                    mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 / val2;
                }
            } break;
            case TY_INST_MOD: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 % val2;
            } break;
            case TY_INST_SHL: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 << val2;
            } break;
            case TY_INST_SHR: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 >> val2;
            } break;
            case TY_INST_BITAND: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 & val2;
            } break;
            case TY_INST_BITOR: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 | val2;
            } break;
            case TY_INST_BITXOR: {
                int32_t val2 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t val1 = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = val1 ^ val2;
            } break;
            case TY_INST_BITNOT: {
                int32_t val = mem.bin[--mem.bin[GLOBALADDR_SP]];
                mem.bin[mem.bin[GLOBALADDR_SP]++] = ~val;
            } break;
            case TY_INST_READ: {
                int32_t ch = 0;
                int32_t fd = mem.bin[--mem.bin[GLOBALADDR_SP]];
                read(fd, &ch, 1);
                mem.bin[mem.bin[GLOBALADDR_SP]++] = ch;
            } break;
            case TY_INST_WRITE: {
                int32_t ch = mem.bin[--mem.bin[GLOBALADDR_SP]];
                int32_t fd = mem.bin[--mem.bin[GLOBALADDR_SP]];
                write(fd, &ch, 1);
            } break;
            default:
                return ERR;
        }
    }
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
    return 0;
}