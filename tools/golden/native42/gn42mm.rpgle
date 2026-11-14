**FREE
// Stage 2, S2.33: %MAX and %MIN of numeric fields with different digits
// and decimal places, three arguments, equal values, negatives, inside
// %CHAR, arithmetic and a condition. Captured on PUB400
// (tools/golden/native42.txt n42_mm) for the native unit test NMINMAX.
ctl-opt dftactgrp(*no);
dcl-s a packed(5:2) inz(1.50);
dcl-s b packed(7:3) inz(2.125);
dcl-s c int(10) inz(-7);
dcl-s d zoned(3:0) inz(2);
dcl-s e packed(4:1) inz(1.5);
dcl-s r packed(9:3);
dcl-s k int(10);
r = %max(a:b);
say('MAX AB ' + %char(r));
r = %min(a:b);
say('MIN AB ' + %char(r));
say('CHAR MAX AB ' + %char(%max(a:b)));
say('CHAR MIN AB ' + %char(%min(a:b)));
say('CHAR MAX AD ' + %char(%max(a:d)));
say('CHAR MAX AE ' + %char(%max(a:e)));
say('CHAR MAX EA ' + %char(%max(e:a)));
r = %max(a:b:c:d);
say('MAX 4 ' + %char(r));
r = %min(a:b:c:d);
say('MIN 4 ' + %char(r));
k = %max(c:-10);
say('MAX NEG ' + %char(k));
r = %max(a:b) * 2 + %min(c:d);
say('MM ARITH ' + %char(r));
if %max(a:d) = 2;
  say('MM COND TRUE');
else;
  say('MM COND FALSE');
endif;
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
