**FREE
// Native unit NOPTTRM (S2.116): OPTIONS(*TRIM), the program captured on
// PUB400 as GN116TRM with DSPLY for its output.
// Source: golden capture tools/golden/native116 n116_trim (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s fld char(10) inz('  xy z');
dcl-s line varchar(200);
line = show('  ab  ') + ' ' + showc(fld) + ' ' + show(%trimr(fld));
dsply (line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(30);
    p varchar(20) value options(*trim);
  end-pi;
  return '[' + p + ']' + %char(%len(p));
end-proc;

dcl-proc showc;
  dcl-pi *n varchar(30);
    p char(8) const options(*trim);
  end-pi;
  return '[' + p + ']';
end-proc;
