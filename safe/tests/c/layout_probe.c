#include <stddef.h>
#include <stdio.h>

#include "csv.h"

int main(void) {
    printf("size=%zu\n", sizeof(struct csv_parser));
    printf("align=%zu\n", (size_t)_Alignof(struct csv_parser));
    printf("pstate=%zu\n", offsetof(struct csv_parser, pstate));
    printf("quoted=%zu\n", offsetof(struct csv_parser, quoted));
    printf("spaces=%zu\n", offsetof(struct csv_parser, spaces));
    printf("entry_buf=%zu\n", offsetof(struct csv_parser, entry_buf));
    printf("entry_pos=%zu\n", offsetof(struct csv_parser, entry_pos));
    printf("entry_size=%zu\n", offsetof(struct csv_parser, entry_size));
    printf("status=%zu\n", offsetof(struct csv_parser, status));
    printf("options=%zu\n", offsetof(struct csv_parser, options));
    printf("quote_char=%zu\n", offsetof(struct csv_parser, quote_char));
    printf("delim_char=%zu\n", offsetof(struct csv_parser, delim_char));
    printf("is_space=%zu\n", offsetof(struct csv_parser, is_space));
    printf("is_term=%zu\n", offsetof(struct csv_parser, is_term));
    printf("blk_size=%zu\n", offsetof(struct csv_parser, blk_size));
    printf("malloc_func=%zu\n", offsetof(struct csv_parser, malloc_func));
    printf("realloc_func=%zu\n", offsetof(struct csv_parser, realloc_func));
    printf("free_func=%zu\n", offsetof(struct csv_parser, free_func));
    return 0;
}
