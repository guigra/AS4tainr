**FREE
// Native unit NREADE (S2.171): READ(E) of database files, the program
// captured on PUB400 as GN171RE with DSPLY for its output.
// Source: golden capture tools/golden/native171 n171_reade (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn164f usage(*input) rename(gn164f : gn164r);
dcl-f gn164k usage(*input) usropn extfile('GN164F') extdesc('GN164F') rename(gn164f : gn164s);
dcl-ds r likerec(gn164r);
dcl-s c int(10);
dcl-s line varchar(80);

read(e) gn164f r;
dow not %eof(gn164f);
  c += 1;
  read(e) gn164f r;
enddo;
line = %char(c) + ' ' + %char(%error) + ' ' + %char(%eof(gn164f));
if c < 0;
  open gn164k;
endif;
monitor;
  read(e) gn164k;
  line += ' ' + %char(%error) + ' ' + %char(%status(gn164k));
on-error;
  line += ' caught ' + %char(%status);
endmon;
dsply (line);
*inlr = *on;
