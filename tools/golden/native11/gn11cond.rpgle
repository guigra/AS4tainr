**FREE
// Stage 2, S2.13: conditions with built-in functions and concatenation
// on either side. Captured on PUB400 (tools/golden/native11.txt n11_cond)
// for the native unit test NCOND.
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCDEFGHIJ');
dcl-s b char(6) inz('  XY  ');
if %scan('C':a) > 0;
  say('SCAN COND YES');
else;
  say('SCAN COND NO');
endif;
if %scan('Q':a) = 0;
  say('SCAN COND NONE');
else;
  say('SCAN COND FOUND');
endif;
if %subst(a:2:3) = 'BCD';
  say('SUBST COND EQ');
else;
  say('SUBST COND NE');
endif;
if %trim(b) = 'XY';
  say('TRIM COND EQ');
else;
  say('TRIM COND NE');
endif;
if %trim(b) + 'Z' = 'XYZ';
  say('CAT COND EQ');
else;
  say('CAT COND NE');
endif;
if %triml(b) = 'XY';
  say('TRIML COND PADDED EQ');
else;
  say('TRIML COND PADDED NE');
endif;
if %scan('C':a) > 2 and %trim(b) <> '';
  say('AND COND YES');
else;
  say('AND COND NO');
endif;
if %scan('Z':a) > 0 or %subst(a:1:1) = 'A';
  say('OR COND YES');
else;
  say('OR COND NO');
endif;
if %len(a) = 10;
  say('LEN COND YES');
else;
  say('LEN COND NO');
endif;
if %xlate('AB':'ab':a) = 'abCDEFGHIJ';
  say('XLATE COND EQ');
else;
  say('XLATE COND NE');
endif;
if %subst(a:1:3) < 'ABD';
  say('SUBST COND LT');
else;
  say('SUBST COND GE');
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
