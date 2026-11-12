**FREE
// EVAL with and without H: a product with more decimal places than the
// target, rounded or cut; and the decimals +, - and * give.
ctl-opt dftactgrp(*no);
dcl-s x packed(5:2) inz(1.27);
dcl-s y packed(5:1) inz(2.5);
dcl-s r1 packed(5:1);
dcl-s r4 packed(9:4);
dcl-s n packed(5:2) inz(-1.27);
dcl-s s packed(7:3);
r1 = x * y;
say('EVAL MUL TRUNC ' + %char(r1));
eval(h) r1 = x * y;
say('EVAL MUL HALF ' + %char(r1));
eval(h) r1 = n * y;
say('EVAL MUL HALF NEG ' + %char(r1));
r4 = x * y;
say('EVAL MUL FULL ' + %char(r4));
s = x + y - 0.001;
say('EVAL ADDSUB ' + %char(s));
eval(h) r1 = 0.05 * 1;
say('EVAL HALF EDGE ' + %char(r1));
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
