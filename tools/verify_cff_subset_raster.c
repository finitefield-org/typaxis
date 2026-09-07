/* Independent FreeType glyph rendering. Pair with verify_harano_cff_subset.py
 * for original source SHA-256 and complete outline/cmap/width verification.
 * Hinting is disabled on both fonts: canonical subsets preserve outlines but
 * intentionally do not copy the source hint programs. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ft2build.h>
#include FT_FREETYPE_H

static void check(FT_Error error, const char *operation) {
    if (error) { fprintf(stderr, "%s: FreeType error %d\n", operation, error); exit(1); }
}
static const unsigned char *row(const FT_Bitmap *b, unsigned y) {
    return b->pitch >= 0 ? b->buffer + (size_t)y * (unsigned)b->pitch
        : b->buffer + (size_t)(b->rows - 1 - y) * (unsigned)(-b->pitch);
}
int main(int argc, char **argv) {
    if (argc != 4) { fprintf(stderr, "usage: %s source.otf subset.otf mapping.gids\n", argv[0]); return 2; }
    FT_Library library; FT_Face source, subset;
    check(FT_Init_FreeType(&library), "init");
    check(FT_New_Face(library, argv[1], 0, &source), "source");
    check(FT_New_Face(library, argv[2], 0, &subset), "subset");
    FILE *mapping = fopen(argv[3], "r");
    if (!mapping) { perror("mapping"); return 1; }
    unsigned original, dense, count = 0, previous = 0, rendered = 0;
    const unsigned sizes[] = {12, 24, 48, 96};
    int status;
    while ((status = fscanf(mapping, "%u %u", &original, &dense)) == 2) {
        if (dense != count || original >= (unsigned)source->num_glyphs || dense >= (unsigned)subset->num_glyphs ||
            (!count && original != 0) || (count && original <= previous)) {
            fprintf(stderr, "invalid mapping\n"); return 1;
        }
        previous = original;
        for (unsigned i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
            check(FT_Set_Pixel_Sizes(source, 0, sizes[i]), "source size");
            check(FT_Set_Pixel_Sizes(subset, 0, sizes[i]), "subset size");
            const FT_Int32 flags = FT_LOAD_NO_HINTING | FT_LOAD_NO_AUTOHINT | FT_LOAD_NO_BITMAP;
            check(FT_Load_Glyph(source, original, flags), "source glyph");
            check(FT_Load_Glyph(subset, dense, flags), "subset glyph");
            check(FT_Render_Glyph(source->glyph, FT_RENDER_MODE_NORMAL), "source render");
            check(FT_Render_Glyph(subset->glyph, FT_RENDER_MODE_NORMAL), "subset render");
            FT_GlyphSlot a = source->glyph, b = subset->glyph;
            if (a->bitmap_left != b->bitmap_left || a->bitmap_top != b->bitmap_top ||
                a->advance.x != b->advance.x || a->advance.y != b->advance.y ||
                a->bitmap.width != b->bitmap.width || a->bitmap.rows != b->bitmap.rows ||
                a->bitmap.pixel_mode != FT_PIXEL_MODE_GRAY || b->bitmap.pixel_mode != FT_PIXEL_MODE_GRAY ||
                a->bitmap.num_grays != b->bitmap.num_grays) {
                fprintf(stderr, "metric/raster shape mismatch GID %u/%u at %u px\n", original, dense, sizes[i]); return 1;
            }
            for (unsigned y = 0; y < a->bitmap.rows; y++) {
                if (memcmp(row(&a->bitmap, y), row(&b->bitmap, y), a->bitmap.width)) {
                    fprintf(stderr, "raster mismatch GID %u/%u at %u px row %u\n", original, dense, sizes[i], y); return 1;
                }
            }
            rendered++;
        }
        count++;
    }
    if (status != EOF || count != (unsigned)subset->num_glyphs || count == 0) { fprintf(stderr, "incomplete mapping\n"); return 1; }
    fclose(mapping);
    FT_Int major, minor, patch; FT_Library_Version(library, &major, &minor, &patch);
    printf("FreeType %d.%d.%d: %u glyphs, %u unhinted raster comparisons passed at 12/24/48/96 px\n", major, minor, patch, count, rendered);
    check(FT_Done_Face(source), "source close"); check(FT_Done_Face(subset), "subset close");
    check(FT_Done_FreeType(library), "done");
    return 0;
}
