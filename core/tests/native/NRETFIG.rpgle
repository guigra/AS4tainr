**FREE
// Native unit NRETFIG (S2.132): RETURN of figurative constants, the
// program captured on PUB400 as GN132RF with DSPLY for its output.
// Source: golden capture tools/golden/native132 n132_retfig (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = %char(isbig(7)) + %char(isbig(2)) + '[' + blank() + ']'
     + %char(zero()) + ' ' + %char(hi()) + ' ' + allx();
dsply (line);
*inlr = *on;

dcl-proc isbig;
  dcl-pi *n ind;
    n int(10) value;
  end-pi;
  if n > 5;
    return *on;
  endif;
  return *off;
end-proc;

dcl-proc blank;
  dcl-pi *n char(3);
  end-pi;
  return *blanks;
end-proc;

dcl-proc zero;
  dcl-pi *n packed(5:2);
  end-pi;
  return *zeros;
end-proc;

dcl-proc hi;
  dcl-pi *n packed(3:0);
  end-pi;
  return *hival;
end-proc;

dcl-proc allx;
  dcl-pi *n char(4);
  end-pi;
  return *all'ab';
end-proc;
