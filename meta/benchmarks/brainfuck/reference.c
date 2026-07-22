#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TAPE_SIZE 30000U

static int fail(const char *message) {
    fprintf(stderr, "reference: %s\n", message);
    return 1;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        return fail("usage: reference PROGRAM.bf");
    }

    FILE *input = fopen(argv[1], "rb");
    if (input == NULL) {
        return fail("cannot open program");
    }
    if (fseek(input, 0, SEEK_END) != 0) {
        fclose(input);
        return fail("cannot seek program");
    }
    long length = ftell(input);
    if (length < 0 || fseek(input, 0, SEEK_SET) != 0) {
        fclose(input);
        return fail("cannot measure program");
    }

    size_t source_length = (size_t)length;
    uint8_t *source = malloc(source_length == 0 ? 1 : source_length);
    uint8_t *code = malloc(source_length == 0 ? 1 : source_length);
    if (source == NULL || code == NULL) {
        fclose(input);
        free(source);
        free(code);
        return fail("allocation failed");
    }
    if (fread(source, 1, source_length, input) != source_length) {
        fclose(input);
        free(source);
        free(code);
        return fail("cannot read program");
    }
    if (fclose(input) != 0) {
        free(source);
        free(code);
        return fail("cannot close program");
    }

    size_t code_length = 0;
    for (size_t index = 0; index < source_length; ++index) {
        if (strchr("><+-.,[]", source[index]) != NULL) {
            code[code_length++] = source[index];
        }
    }
    free(source);

    size_t *jumps = calloc(code_length == 0 ? 1 : code_length, sizeof(size_t));
    size_t *stack = malloc((code_length == 0 ? 1 : code_length) * sizeof(size_t));
    uint8_t *tape = calloc(TAPE_SIZE, sizeof(uint8_t));
    if (jumps == NULL || stack == NULL || tape == NULL) {
        free(code);
        free(jumps);
        free(stack);
        free(tape);
        return fail("allocation failed");
    }

    size_t depth = 0;
    for (size_t index = 0; index < code_length; ++index) {
        if (code[index] == '[') {
            stack[depth++] = index;
        } else if (code[index] == ']') {
            if (depth == 0) {
                free(code);
                free(jumps);
                free(stack);
                free(tape);
                return fail("unmatched ]");
            }
            size_t open = stack[--depth];
            jumps[open] = index;
            jumps[index] = open;
        }
    }
    if (depth != 0) {
        free(code);
        free(jumps);
        free(stack);
        free(tape);
        return fail("unmatched [");
    }
    free(stack);

    size_t pc = 0;
    size_t pointer = 0;
    while (pc < code_length) {
        switch (code[pc]) {
        case '>':
            if (pointer + 1 >= TAPE_SIZE) {
                return fail("tape pointer overflow");
            }
            ++pointer;
            break;
        case '<':
            if (pointer == 0) {
                return fail("tape pointer underflow");
            }
            --pointer;
            break;
        case '+':
            tape[pointer] = (uint8_t)(tape[pointer] + 1U);
            break;
        case '-':
            tape[pointer] = (uint8_t)(tape[pointer] - 1U);
            break;
        case '.':
            if (putchar(tape[pointer]) == EOF) {
                return fail("output failed");
            }
            break;
        case ',': {
            int byte = getchar();
            tape[pointer] = byte == EOF ? 0 : (uint8_t)byte;
            break;
        }
        case '[':
            if (tape[pointer] == 0) {
                pc = jumps[pc];
            }
            break;
        case ']':
            if (tape[pointer] != 0) {
                pc = jumps[pc];
            }
            break;
        default:
            return fail("internal invalid command");
        }
        ++pc;
    }

    int status = fflush(stdout) == 0 ? 0 : fail("output flush failed");
    free(code);
    free(jumps);
    free(tape);
    return status;
}
