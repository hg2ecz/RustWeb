TEX=main.tex
PDF=rwlang-webfejlesztoknek-hu.pdf
LATEXMK=latexmk

.PHONY: all pdf clean distclean

all: pdf

pdf:
	$(LATEXMK) -pdf -interaction=nonstopmode -halt-on-error -jobname=rwlang-webfejlesztoknek-hu $(TEX)

clean:
	$(LATEXMK) -c -jobname=rwlang-webfejlesztoknek-hu $(TEX)

# A PDF is a build artifact too; distclean leaves only editable sources.
distclean:
	$(LATEXMK) -C -jobname=rwlang-webfejlesztoknek-hu $(TEX)
	rm -f $(PDF)
