// typaxis-math-fixtures generates actual engine SVG and derived Safe-SVG pairs
// for local admission/rendering gates. It is not the VMB book renderer.
package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"

	"github.com/finitefield/vmb-core/internal/rendermath"
	"github.com/finitefield/vmb-core/internal/rendertypaxis"
)

type metrics struct {
	Advance  int64    `json:"advance"`
	Ascent   int64    `json:"ascent"`
	Descent  int64    `json:"descent"`
	OriginX  int64    `json:"origin_x"`
	Baseline int64    `json:"baseline"`
	Viewport viewport `json:"viewport"`
}
type viewport struct {
	Width  int64 `json:"width"`
	Height int64 `json:"height"`
}
type fixture struct {
	CaseID        string                      `json:"case_id"`
	Display       string                      `json:"display"`
	TeX           string                      `json:"tex"`
	Speech        string                      `json:"speech"`
	FontSizeRaw   int64                       `json:"font_size_raw"`
	SourceSVG     string                      `json:"source_svg"`
	SourceSHA256  string                      `json:"source_sha256"`
	DerivedSVG    string                      `json:"derived_svg"`
	DerivedSHA256 string                      `json:"derived_sha256"`
	Metrics       metrics                     `json:"metrics"`
	PathCount     uint64                      `json:"path_count"`
	SegmentCount  uint64                      `json:"segment_count"`
	Engine        rendermath.ArtifactIdentity `json:"engine_artifact"`
}

func main() {
	output := flag.String("output", "", "new output directory (must not exist)")
	corpus := flag.String("corpus", "small", "small or distinct-5000")
	flag.Parse()
	if *output == "" || flag.NArg() != 0 || (*corpus != "small" && *corpus != "distinct-5000") {
		fmt.Fprintln(os.Stderr, "usage: typaxis-math-fixtures -output NEW_DIRECTORY [-corpus small|distinct-5000]")
		os.Exit(2)
	}
	generate := run
	if *corpus == "distinct-5000" {
		generate = runDistinct
	}
	if err := generate(*output); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(output string) error {
	ctx := context.Background()
	engine, err := rendermath.NewEngine(ctx, rendermath.ProductionEngineOptions(rendermath.EngineBuild, ""))
	if err != nil {
		return err
	}
	defer engine.Close(ctx)
	cases := []struct{ id, tex, speech string }{
		{"fraction", `\frac{1}{2}=\frac{2}{4}`, "one half equals two quarters"},
		{"parentheses", `\left(\frac{a}{b}\right)`, "a over b in parentheses"},
		{"equivalence", `a\iff b`, "a if and only if b"},
		{"negation", `\neg P`, "not P"},
		{"long", `a+b+c+d+e+f+g+h+i+j+k+l+m+n`, "the sum of a through n"},
	}
	var records []fixture
	type file struct {
		name  string
		bytes []byte
	}
	var files []file
	for _, c := range cases {
		for _, display := range []rendermath.DisplayMode{rendermath.DisplayInline, rendermath.DisplayBlock} {
			mode := "inline"
			if display == rendermath.DisplayBlock {
				mode = "block"
			}
			r, err := engine.Render(ctx, rendermath.RenderRequest{TeX: c.tex, Display: display, RootSyntax: rendermath.RootMath, Speech: c.speech, SpeechSource: rendermath.SpeechSemantic, Locale: "en", TextDirection: rendermath.DirectionLTR, Accessibility: rendermath.AccessibilityRequireSpeech, Fallback: rendermath.FallbackForbidden})
			if err != nil {
				return err
			}
			stem := c.id + "-" + mode
			sourceName := stem + ".source.svg"
			files = append(files, file{sourceName, r.SVG})
			for _, size := range []int64{11 * 65536, 786431} {
				g, err := rendertypaxis.LowerMathVector(rendertypaxis.MathVectorInput{Result: r, FontSizeRaw: size})
				if err != nil {
					return err
				}
				derivedName := fmt.Sprintf("%s-%d.svg", stem, size)
				files = append(files, file{derivedName, g.SVG})
				records = append(records, fixture{c.id, mode, c.tex, c.speech, size, sourceName, digest(r.SVG), derivedName, digest(g.SVG),
					metrics{g.Advance, g.Ascent, g.Descent, g.OriginX, g.Baseline, viewport{g.ViewportWidth, g.ViewportHeight}}, g.PathCount, g.SegmentCount, r.Artifact})
			}
		}
	}
	manifest, err := json.MarshalIndent(struct {
		Algorithm    string    `json:"algorithm"`
		CorpusKind   string    `json:"corpus_kind"`
		RulesVersion string    `json:"rules_version"`
		Cases        []fixture `json:"cases"`
	}{"vmb.typaxis-engine-fixtures/1", "authored test expressions rendered by the actual VMB engine", rendertypaxis.AdapterRulesID, records}, "", "  ")
	if err != nil {
		return err
	}
	// Create only a new directory. A manifest is written last so incomplete
	// generations cannot be mistaken for a complete fixture inventory.
	if err := os.Mkdir(output, 0700); err != nil {
		return err
	}
	for _, f := range files {
		if err := os.WriteFile(filepath.Join(output, f.name), f.bytes, 0600); err != nil {
			return err
		}
	}
	if err := os.WriteFile(filepath.Join(output, "fixture-index.json"), append(manifest, '\n'), 0600); err != nil {
		return err
	}
	fmt.Printf("generated %d cases and %d SVG files\n", len(records), len(files))
	return nil
}
func digest(data []byte) string { h := sha256.Sum256(data); return hex.EncodeToString(h[:]) }
