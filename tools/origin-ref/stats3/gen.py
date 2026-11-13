import sys
def dump(book, sheet, nc=18, nr=8, tag=''):
    s=(f"page.active$ = {sheet};\n" if sheet else "")+f"#DUMP {tag} {book}!{sheet} {nr}\n"
    for c in range(1,nc+1):
        for r in range(1,nr+1):
            s+=f"double dv = wcol({c})[{r}];\n?dv\n"
            s+=f"string ds$ = wcol({c})[{r}]$;\n$ds$\n"
    return s
def cols(cols_):
    s="newbook;\n"
    for i,c in enumerate(cols_):
        vals=', '.join('0' if v is None else repr(float(v)) for v in c)
        if i>0: s+="wks.addcol(); "
        s+=f"col({i+1}) = {{{vals}}};\n"
        for r,v in enumerate(c):
            if v is None: s+=f"col({i+1})[{r+1}] = 0/0;\n"
    return s
