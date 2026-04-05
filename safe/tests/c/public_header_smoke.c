#include <csv.h>

static int is_space(unsigned char byte) {
    return byte == CSV_SPACE;
}

static int is_term(unsigned char byte) {
    return byte == '\n';
}

static void *passthrough_realloc(void *ptr, size_t size) {
    return realloc(ptr, size);
}

static void passthrough_free(void *ptr) {
    free(ptr);
}

static void field_cb(void *field, size_t len, void *data) {
    (void)field;
    (void)len;
    (void)data;
}

static void row_cb(int term, void *data) {
    (void)term;
    (void)data;
}

int main(void) {
    struct csv_parser parser;
    unsigned char field[] = "x";
    unsigned char dest[8];

    if (csv_init(&parser, CSV_APPEND_NULL) != 0) {
        return 1;
    }

    csv_set_opts(&parser, CSV_EMPTY_IS_NULL);
    csv_set_delim(&parser, ';');
    csv_set_quote(&parser, '\'');
    csv_set_space_func(&parser, is_space);
    csv_set_term_func(&parser, is_term);
    csv_set_realloc_func(&parser, passthrough_realloc);
    csv_set_free_func(&parser, passthrough_free);
    csv_set_blk_size(&parser, 8);

    (void)csv_get_opts(&parser);
    (void)csv_get_delim(&parser);
    (void)csv_get_quote(&parser);
    (void)csv_get_buffer_size(&parser);
    (void)csv_parse(&parser, field, sizeof(field) - 1, field_cb, row_cb, NULL);
    (void)csv_fini(&parser, field_cb, row_cb, NULL);
    (void)csv_error(&parser);
    (void)csv_strerror(CSV_SUCCESS);
    (void)csv_write(dest, sizeof(dest), field, sizeof(field) - 1);
    (void)csv_write2(dest, sizeof(dest), field, sizeof(field) - 1, '\'');
    (void)csv_fwrite(NULL, field, sizeof(field) - 1);
    (void)csv_fwrite2(NULL, field, sizeof(field) - 1, '\'');

    csv_free(&parser);
    return 0;
}
