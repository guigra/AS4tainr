**FREE
// Native unit NLENSET (S2.102): %LEN as the target of an assignment,
// the program captured on PUB400 as GN102LEN with DSPLY for its output.
// Source: golden capture tools/golden/native102 n102_lenset (PUB400, IBM
// i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s v varchar(8) inz('abcdef');
dcl-s line varchar(80);
%len(v) = 3;
line = '[' + v + ']';
%len(v) = %len(v) + 2;
line += '[' + v + ']';
%len(v) = 0;
line += '[' + v + ']' + %char(%len(v));
monitor;
  %len(v) = 9;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
line += ' ' + local();
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s w varchar(6) inz('xyz');
  %len(w) = 1;
  return w;
end-proc;
