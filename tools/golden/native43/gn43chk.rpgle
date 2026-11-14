**FREE
// Stage 2, S2.34: %CHECK and %CHECKR with no start, of fixed-length and
// varying fields, expressions and literals: every character valid, the
// first or last not, trailing blanks, an empty string, in arithmetic and
// a condition. Captured on PUB400 (tools/golden/native43.txt n43_chk) for
// the native unit test NCHECK.
ctl-opt dftactgrp(*no);
dcl-c DIGITS '0123456789';
dcl-s c char(10) inz('0012345');
dcl-s v varchar(10) inz('12a45');
dcl-s e varchar(10) inz('');
dcl-s b char(8) inz('  ab  ');
dcl-s n int(10);
n = %check(DIGITS:c);
say('CHK C ' + %char(n));
n = %checkr(DIGITS:c);
say('CHKR C ' + %char(n));
n = %checkr(' ':c);
say('CHKR BLANK C ' + %char(n));
n = %check(DIGITS:v);
say('CHK V ' + %char(n));
n = %checkr(DIGITS:v);
say('CHKR V ' + %char(n));
n = %check(DIGITS:'2024');
say('CHK ALL ' + %char(n));
n = %check(DIGITS:e);
say('CHK EMPTY ' + %char(n));
n = %checkr(DIGITS:e);
say('CHKR EMPTY ' + %char(n));
n = %check(' ':b);
say('CHK LEAD ' + %char(n));
n = %checkr(' ':b);
say('CHKR TRAIL ' + %char(n));
n = %check('a':%trim(b) + 'xa');
say('CHK EXPR ' + %char(n));
n = %checkr(' ':c) - %check(DIGITS:v) * 2;
say('CHK ARITH ' + %char(n));
if %check(DIGITS:v) > 0;
  say('CHK COND TRUE');
else;
  say('CHK COND FALSE');
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
