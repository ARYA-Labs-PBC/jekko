LATEXMK ?= latexmk
PDFINFO ?= pdfinfo
PDFTOTEXT ?= pdftotext
RG ?= rg
DRAFT_MARKERS := T(O)DO|P(LACEHOLDER)|citation[[:space:]]needed|[?][?]|over[c]laim

.PHONY: pdf check clean

pdf:
	PATH="$(CURDIR)/bin:$$PATH" $(LATEXMK) -pdf -shell-escape -interaction=nonstopmode -halt-on-error -file-line-error -outdir=build ZYAL.tex
	cp build/ZYAL.pdf ZYAL.pdf

check: pdf
	test -f ZYAL.pdf
	pages="$$( $(PDFINFO) ZYAL.pdf | awk '/^Pages:/ {print $$2}' )"; \
	  test "$$pages" -ge 8 -a "$$pages" -le 20
	$(PDFTOTEXT) ZYAL.pdf - | $(RG) "ZYAL|LangGraph|OpenHands|Hermes|References|Appendix" >/dev/null
	! $(RG) -n "$(DRAFT_MARKERS)" ZYAL.tex ZYAL.bib research figures listings
	cd ../packages/jekko && bun test src/agent-script/parser.test.ts

clean:
	$(LATEXMK) -C -outdir=build ZYAL.tex || true
	rm -rf build _minted-ZYAL ZYAL.pdf
