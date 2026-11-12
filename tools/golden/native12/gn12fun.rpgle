**FREE
// Stage 2, S2.14: %CHAR of numbers, %INT, %INTH, %DEC and %DECH.
// Captured on PUB400 (tools/golden/native12.txt n12_fun) for the native
// unit test NFUN.
ctl-opt dftactgrp(*no);
dcl-s d52 packed(5:2) inz(-1.50);
dcl-s p70 packed(7:0) inz(1234);
dcl-s z zoned(5:2) inz(0.25);
dcl-s c char(12);
dcl-s n packed(5:0);
dcl-s x packed(5:1);
c = %char(d52);
say('CHAR NEG [' + c + ']');
c = %char(p70);
say('CHAR INT [' + c + ']');
c = %char(z);
say('CHAR FRACTION [' + c + ']');
c = %char(p70 * 2);
say('CHAR EXPR [' + c + ']');
c = %char(%int(d52));
say('CHAR INT FN [' + c + ']');
c = %char(%inth(d52));
say('CHAR INTH FN [' + c + ']');
c = 'X' + %char(3);
say('CHAR LIT [' + c + ']');
n = %int(z * 10);
say('INT EXPR ' + %char(n));
n = %inth(z * 10);
say('INTH EXPR ' + %char(n));
x = %dec(d52 : 5 : 1);
say('DEC ' + %char(x));
x = %dech(1.25 : 5 : 1);
say('DECH ' + %char(x));
monitor;
  x = %dec(p70 : 3 : 0);
  say('DEC SMALL ' + %char(x));
on-error;
  say('DEC SMALL ERR ' + %char(%status));
endmon;
if %int(d52 * 2) = -3;
  say('INT COND YES');
else;
  say('INT COND NO');
endif;
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
