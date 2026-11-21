**FREE
// Native unit NOVERLD (S2.188): calls of an OVERLOAD prototype, the
// program captured on PUB400 as GN188OV with DSPLY for its output.
// Source: golden capture tools/golden/native188 n188_overload (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-03-04');
dcl-s c char(5) inz('abc');
dcl-s i int(10) inz(-7);
dcl-s line varchar(200);
// The OVERLOAD prototype has no END-PR (an END-PR is RNF3551 on IBM i);
// a prototype follows it (the interpreter ends it only there).
dcl-pr fmt varchar(30) overload(fmtNum : fmtDate : fmtChr);
dcl-pr fmtNum varchar(30);
  n packed(9:2) const;
end-pr;

// One statement per call: each runs natively or not on its own.
line = fmt(12.5);
line += '|' + fmt(d);
line += '|' + fmt(c);
line += '|' + fmt(i);
dsply (line);
*inlr = *on;

dcl-proc fmtNum;
  dcl-pi *n varchar(30);
    n packed(9:2) const;
  end-pi;
  return 'N' + %char(n);
end-proc;

dcl-proc fmtDate;
  dcl-pi *n varchar(30);
    x date const;
  end-pi;
  return 'D' + %char(x : *dmy);
end-proc;

dcl-proc fmtChr;
  dcl-pi *n varchar(30);
    s varchar(20) const;
  end-pi;
  return 'C' + %trim(s);
end-proc;
