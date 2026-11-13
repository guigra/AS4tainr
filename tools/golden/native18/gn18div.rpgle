**FREE
// Stage 2, S2.17: / in expressions: the quotient's decimal places, EVAL
// and EVAL(H), division by zero. Captured on PUB400
// (tools/golden/native18.txt n18_div) for the native unit test NDIVE.
ctl-opt dftactgrp(*no);
dcl-s p72 packed(7:2) inz(10);
dcl-s i10 int(10) inz(10);
dcl-s q packed(9:2);
dcl-s z zoned(5:0) inz(0);
dcl-s c char(40);
c = %char(10 / 3);
say('DIV LIT [' + c + ']');
c = %char(100 / 3);
say('DIV LIT100 [' + c + ']');
c = %char(p72 / 3);
say('DIV PACKED [' + c + ']');
c = %char(i10 / 3);
say('DIV INT [' + c + ']');
q = p72 / 3;
say('DIV ASSIGN ' + %char(q));
eval(h) q = 2 / 3;
say('DIV HALF ' + %char(q));
q = (p72 + 5) / 4 * 2;
say('DIV EXPR ' + %char(q));
monitor;
  q = p72 / z;
  say('DIV ZERO ' + %char(q));
on-error 00102;
  say('DIV ZERO 00102');
endmon;
*inlr = *on;
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
