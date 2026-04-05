#include "common.h"

enum {
    FIELD_BEGUN = 2
};

struct event_state {
    size_t field_count;
    size_t row_count;
    int last_term;
    size_t first_len;
    size_t second_len;
    unsigned char first_field[8];
    unsigned char second_field[8];
};

static size_t failing_realloc_calls;
static size_t working_realloc_calls;

static void *failing_growth_realloc(void *ptr, size_t size) {
    failing_realloc_calls++;
    if (size > 2) {
        return NULL;
    }
    return realloc(ptr, size);
}

static void *working_realloc(void *ptr, size_t size) {
    working_realloc_calls++;
    return realloc(ptr, size);
}

static void field_cb(void *field, size_t len, void *data) {
    struct event_state *state = (struct event_state *)data;

    ASSERT(field != NULL);
    if (state->field_count == 0) {
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
    struct event_state *state = (struct event_state *)data;

    state->row_count++;
    state->last_term = term;
}

int main(void) {
    struct csv_parser parser;
    struct event_state state;
    unsigned char input[] = "ab,cd";
    size_t consumed;

    ASSERT_INT_EQ(csv_init(&parser, 0), 0);
    csv_set_blk_size(&parser, 2);
    csv_set_realloc_func(&parser, failing_growth_realloc);

    memset(&state, 0, sizeof(state));
    consumed = csv_parse(
        &parser,
        input,
        sizeof(input) - 1,
        field_cb,
        row_cb,
        &state
    );
    ASSERT_SIZE_EQ(consumed, 2);
    ASSERT_INT_EQ(csv_error(&parser), CSV_ENOMEM);
    ASSERT_SIZE_EQ(state.field_count, 0);
    ASSERT_SIZE_EQ(state.row_count, 0);
    ASSERT_INT_EQ(parser.pstate, FIELD_BEGUN);
    ASSERT_INT_EQ(parser.quoted, 0);
    ASSERT_SIZE_EQ(parser.spaces, 0);
    ASSERT_SIZE_EQ(parser.entry_pos, 2);
    ASSERT_SIZE_EQ(parser.entry_size, 2);
    ASSERT(parser.entry_buf != NULL);
    ASSERT_BYTES_EQ(parser.entry_buf, "ab", 2);
    ASSERT(failing_realloc_calls >= 3);

    csv_set_realloc_func(&parser, working_realloc);
    consumed = csv_parse(
        &parser,
        input + consumed,
        (sizeof(input) - 1) - consumed,
        field_cb,
        row_cb,
        &state
    );
    ASSERT_SIZE_EQ(consumed, 3);
    ASSERT(working_realloc_calls >= 1);
    ASSERT_INT_EQ(csv_fini(&parser, field_cb, row_cb, &state), 0);
    ASSERT_INT_EQ(csv_error(&parser), CSV_SUCCESS);
    ASSERT_SIZE_EQ(state.field_count, 2);
    ASSERT_SIZE_EQ(state.row_count, 1);
    ASSERT_INT_EQ(state.last_term, -1);
    ASSERT_BYTES_EQ(state.first_field, "ab", 2);
    ASSERT_BYTES_EQ(state.second_field, "cd", 2);
    csv_free(&parser);

    ASSERT_INT_EQ(csv_init(&parser, 0), 0);
    parser.entry_buf = (unsigned char *)malloc(1);
    ASSERT(parser.entry_buf != NULL);
    parser.entry_size = (size_t)-1;
    parser.entry_pos = (size_t)-1;
    parser.pstate = FIELD_BEGUN;
    parser.quoted = 1;
    parser.spaces = 7;
    parser.status = CSV_SUCCESS;
    parser.blk_size = 1;

    memset(&state, 0, sizeof(state));
    consumed = csv_parse(&parser, "x", 1, field_cb, row_cb, &state);
    ASSERT_SIZE_EQ(consumed, 0);
    ASSERT_INT_EQ(csv_error(&parser), CSV_ETOOBIG);
    ASSERT_SIZE_EQ(state.field_count, 0);
    ASSERT_SIZE_EQ(state.row_count, 0);
    ASSERT_INT_EQ(parser.pstate, FIELD_BEGUN);
    ASSERT_INT_EQ(parser.quoted, 1);
    ASSERT_SIZE_EQ(parser.spaces, 7);
    ASSERT_SIZE_EQ(parser.entry_pos, (size_t)-1);
    ASSERT_SIZE_EQ(parser.entry_size, (size_t)-1);
    csv_free(&parser);

    return EXIT_SUCCESS;
}
