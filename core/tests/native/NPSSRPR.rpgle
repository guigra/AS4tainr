**FREE
// Native unit NPSSRPR (S2.142): procedures with their own *PSSR, the
// program captured on PUB400 as GN142PS with DSPLY for its output.
// Source: golden capture tools/golden/native142 n142_pssr (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = %char(safediv(10: 2)) + ' ' + %char(safediv(1: 0))
     + ' ' + %char(divstat(1: 0)) + ' ' + %char(divstat(9: 3));
dsply (line);
*inlr = *on;

dcl-proc safediv;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s r int(10);
  r = a / b;
  return r;
  begsr *pssr;
    return -1;
  endsr;
end-proc;

dcl-proc divstat;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s r int(10);
  r = a / b;
  return r;
  begsr *pssr;
    return %status;
  endsr;
end-proc;
