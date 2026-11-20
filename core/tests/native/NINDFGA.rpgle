**FREE
// Native unit NINDFGA (S2.151): *ON and *OFF for an indicator parameter
// passed by value, the program captured on PUB400 as GN151FI with DSPLY
// for its output.
// Source: golden capture tools/golden/native151 n151_figind (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = flag(*on) + flag(*off);
if flag(*off) = 'no';
  line += ' if';
endif;
line += ' ' + %char(count(*on) + count(*off) * 10);
dsply (line);
*inlr = *on;

dcl-proc flag;
  dcl-pi *n varchar(10);
    f ind value;
  end-pi;
  if f;
    return 'yes';
  endif;
  return 'no';
end-proc;

dcl-proc count;
  dcl-pi *n int(10);
    f ind value;
  end-pi;
  if f = *on;
    return 1;
  endif;
  return 2;
end-proc;
