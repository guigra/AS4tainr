**FREE
// Native unit NLIKEF (S2.164): file operations beside a LIKEFILE
// parameter, the program captured on PUB400 as GN164LF with DSPLY for its
// output.
// Source: golden capture tools/golden/native164 n164_likef (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn164f template rename(gn164f : gn164r) block(*no);
dcl-f t1 likefile(gn164f) usropn extfile(fname);
dcl-ds rec likerec(gn164r);
dcl-s fname char(21) inz('GN164F');
dcl-s n int(10);
dcl-s line varchar(80);

open t1;
n = countAll(t1);
line = %char(n);
setll *hival t1;
readp t1 rec;
line += ' ' + %char(rec.n) + rec.s;
close t1;
dsply (line);
*inlr = *on;

dcl-proc countAll;
  dcl-pi *n int(10);
    f likefile(gn164f);
  end-pi;
  dcl-ds r likerec(gn164r);
  dcl-s c int(10);
  setll *start f;
  read f r;
  dow not %eof(f);
    c += 1;
    read f r;
  enddo;
  return c;
end-proc;
