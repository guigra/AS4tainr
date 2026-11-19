**FREE
// Native unit NINDCALL (S2.133): calls returning indicators as
// conditions, the program captured on PUB400 as GN133IC with DSPLY for
// its output.
// Source: golden capture tools/golden/native133 n133_indcall (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s n int(10);
dcl-s line varchar(80);

if isbig(7);
  line += 'a';
endif;
if not isbig(2);
  line += 'b';
endif;
if isbig(9) and isbig(1);
  line += 'c';
endif;
dow not isbig(n);
  n += 2;
enddo;
line += %char(n) + %char(isbig(n));
dsply (line);
*inlr = *on;

dcl-proc isbig;
  dcl-pi *n ind;
    x int(10) value;
  end-pi;
  return x > 5;
end-proc;
