**FREE
// Native unit NONEXIT2 (S2.90): ON-EXIT (a normal end, a RETURN in
// ON-EXIT, an abnormal end caught by the caller). The program captured on
// PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native90 n90_onexit (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s trace varchar(40);
dcl-s r int(10);
dcl-s line varchar(52);
r = work(10: 2);
line = 'A ' + %char(r) + ' ' + trace;
r = work(7: 7);
line += ' B ' + %char(r) + ' ' + trace;
monitor;
  r = work(1: 0);
  line += ' C ' + %char(r);
on-error;
  line += ' ERR ' + %char(%status) + ' ' + trace;
endmon;
dsply (line);
*inlr = *on;

dcl-proc work;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s bad ind;
  trace = 'in';
  return a / b;
on-exit bad;
  if bad;
    trace = 'abnormal';
  else;
    trace = 'normal';
  endif;
  if a = b;
    return 99;
  endif;
end-proc;
