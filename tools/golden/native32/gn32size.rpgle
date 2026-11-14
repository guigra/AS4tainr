**FREE
// Stage 2, S2.25: %SIZE and %ELEM of fields, arrays, data structures and
// named constants. Captured on PUB400 (tools/golden/native32.txt n32_size)
// for the native unit test NSIZE.
ctl-opt dftactgrp(*no);
dcl-s c char(7);
dcl-s p packed(7:2);
dcl-s i int(10);
dcl-s arr char(3) dim(5);
dcl-ds d;
  a char(2);
  b packed(5:0);
end-ds;
dcl-c KCHR 'ABCD';
dcl-c KNUM 12345;
dcl-s n int(10);
n = %size(c);
say('SIZE CHAR ' + %char(n));
n = %size(p);
say('SIZE PACKED ' + %char(n));
n = %size(i);
say('SIZE INT ' + %char(n));
n = %size(arr);
say('SIZE ELEMENT ' + %char(n));
n = %size(arr:*all);
say('SIZE ALL ' + %char(n));
n = %size(d);
say('SIZE DS ' + %char(n));
n = %size(KCHR);
say('SIZE CONST CHAR ' + %char(n));
n = %size(KNUM);
say('SIZE CONST NUM ' + %char(n));
n = %elem(arr);
say('ELEM ' + %char(n));
n = %size(arr:*all) - %elem(arr);
say('SIZE EXPR ' + %char(n));
if %size(c) = 7;
  say('SIZE COND YES');
else;
  say('SIZE COND NO');
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
