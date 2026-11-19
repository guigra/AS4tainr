**FREE
// Native unit NRETPTR (S2.99): calls that return a pointer, the program
// captured on PUB400 as GN100RP with DSPLY for its output.
// Source: golden capture tools/golden/native100 n100_retptr (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no) bnddir('QC2LE');
dcl-pr strstr pointer extproc('strstr');
  s pointer value;
  t pointer value;
end-pr;
dcl-s buf char(12);
dcl-s key char(3);
dcl-s nokey char(3);
dcl-s p pointer;
dcl-s q pointer;
dcl-s line varchar(80);
buf = 'hello world' + x'00';
key = 'wo' + x'00';
nokey = 'zz' + x'00';
p = at(6);
line = %str(p);
p = at(0) + 2;
line += '|' + %str(p: 3);
q = strstr(%addr(buf): %addr(key));
line += '|' + %str(q);
q = strstr(%addr(buf): %addr(nokey));
if q = *null;
  line += '|null';
endif;
dsply (line);
*inlr = *on;

dcl-proc at;
  dcl-pi *n pointer;
    n int(10) value;
  end-pi;
  return %addr(buf) + n;
end-proc;
