**FREE
// Interpreter fix (fork branch fix/compile-checks): %OCCUR set to a
// literal past the OCCURS of the data structure. Expected not to compile
// (RNF7571). Captured on PUB400 (tools/golden/native217.txt).
ctl-opt dftactgrp(*no);
dcl-ds rec occurs(3);
  f char(2);
end-ds;

%occur(rec) = 4;
*inlr = *on;
