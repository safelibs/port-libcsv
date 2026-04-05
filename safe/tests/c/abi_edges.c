#include "common.h"

struct callback_state {
    struct csv_parser *parser;
    size_t field_count;
    size_t row_count;
    int last_term;
    int saw_live_buffer;
    int saw_mutation;
    size_t first_len;
    size_t second_len;
    unsigned char first_field[8];
    unsigned char second_field[8];
};

static size_t realloc_calls;
static size_t free_calls;

static void *counting_realloc(void *ptr, size_t size) {
    realloc_calls++;
    return realloc(ptr, size);
}

static void counting_free(void *ptr) {
    free_calls++;
    free(ptr);
}

static void field_cb(void *field, size_t len, void *data) {
    struct callback_state *state = (struct callback_state *)data;

    ASSERT(field != NULL);
    if (state->field_count == 0) {
        ASSERT(field == state->parser->entry_buf);
        state->saw_live_buffer = 1;
        if (len > 0) {
            ((unsigned char *)field)[0] = 'X';
            state->saw_mutation = state->parser->entry_buf[0] == 'X';
        }
        ASSERT(len <= sizeof(state->first_field));
        memcpy(state->first_field, field, len);
        state->first_len = len;
    } else {
        ASSERT(state->field_count == 1);
        ASSERT(len <= sizeof(state->second_field));
        memcpy(state->second_field, field, len);
        state->second_len = len;
    }

    state->field_count++;
}

static void row_cb(int term, void *data) {
    struct callback_state *state = (struct callback_state *)data;

    state->row_count++;
    state->last_term = term;
}

int main(void) {
    struct csv_parser parser;
    struct callback_state state;
    FILE *fp;
    unsigned char parse_input[] = "ab,cd";
    unsigned char write_input[] = "a\"b";
    unsigned char write2_input[] = "a'b";
    int eof_byte;

    ASSERT_INT_EQ(csv_get_opts(NULL), -1);
    ASSERT_INT_EQ(csv_set_opts(NULL, 0), -1);
    ASSERT_SIZE_EQ(csv_get_buffer_size(NULL), 0);

    ASSERT_SIZE_EQ(csv_write(NULL, 0, write_input, sizeof(write_input) - 1), 6);
    ASSERT_SIZE_EQ(
        csv_write2(NULL, 0, write2_input, sizeof(write2_input) - 1, '\''),
        6
    );

    ASSERT_INT_EQ(csv_fwrite(NULL, write_input, sizeof(write_input) - 1), 0);
    ASSERT_INT_EQ(csv_fwrite2(NULL, write2_input, sizeof(write2_input) - 1, '\''), 0);

    fp = tmpfile();
    ASSERT(fp != NULL);
    ASSERT_INT_EQ(csv_fwrite(fp, NULL, sizeof(write_input) - 1), 0);
    ASSERT_INT_EQ(csv_fwrite2(fp, NULL, sizeof(write2_input) - 1, '\''), 0);
    rewind(fp);
    eof_byte = fgetc(fp);
    ASSERT_INT_EQ(eof_byte, EOF);
    ASSERT_INT_EQ(fclose(fp), 0);

    ASSERT_INT_EQ(csv_init(&parser, 0), 0);
    csv_set_blk_size(&parser, 2);
    csv_set_realloc_func(&parser, counting_realloc);
    csv_set_free_func(&parser, counting_free);
    ASSERT_INT_EQ(csv_set_opts(&parser, CSV_APPEND_NULL), 0);
    ASSERT_INT_EQ(csv_get_opts(&parser), CSV_APPEND_NULL);

    memset(&state, 0, sizeof(state));
    state.parser = &parser;

    ASSERT_SIZE_EQ(
        csv_parse(
            &parser,
            parse_input,
            sizeof(parse_input) - 1,
            field_cb,
            row_cb,
            &state
        ),
        sizeof(parse_input) - 1
    );
    ASSERT_INT_EQ(csv_fini(&parser, field_cb, row_cb, &state), 0);

    ASSERT_SIZE_EQ(state.field_count, 2);
    ASSERT_SIZE_EQ(state.row_count, 1);
    ASSERT_INT_EQ(state.last_term, -1);
    ASSERT(state.saw_live_buffer);
    ASSERT(state.saw_mutation);
    ASSERT_BYTES_EQ(state.first_field, "Xb", 2);
    ASSERT_BYTES_EQ(state.second_field, "cd", 2);
    ASSERT(realloc_calls >= 2);
    ASSERT(csv_get_buffer_size(&parser) >= 4);

    csv_free(&parser);
    ASSERT_SIZE_EQ(csv_get_buffer_size(&parser), 0);
    ASSERT_SIZE_EQ(free_calls, 1);

    return EXIT_SUCCESS;
}
