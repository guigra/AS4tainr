**FREE
// Native unit NSTRING (S2.66): OPTIONS(*STRING) for C functions. A
// literal, a fixed character field (its trailing blanks included), %TRIM
// of it, a varying field, a concatenation and a hexadecimal literal as
// null-terminated strings for strlen, atoi, strcmp and strncmp. The
// program captured on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native66 n66_str (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no) bnddir('QC2LE');
dcl-pr strlen int(10) extproc('strlen');
  s pointer value options(*string);
end-pr;
dcl-pr atoi int(10) extproc('atoi');
  s pointer value options(*string);
end-pr;
dcl-pr strcmp int(10) extproc('strcmp');
  a pointer value options(*string);
  b pointer value options(*string);
end-pr;
dcl-pr strncmp int(10) extproc('strncmp');
  a pointer value options(*string);
  b pointer value options(*string);
  n uns(10) value;
end-pr;
dcl-s fixed char(10) inz('abc');
dcl-s vary varchar(20) inz('xyz12');
dcl-s n int(10);
n = strlen('hello');
dsply ('LEN LIT ' + %char(n));
n = strlen(fixed);
dsply ('LEN FIXED ' + %char(n));
n = strlen(%trim(fixed));
dsply ('LEN TRIM ' + %char(n));
n = strlen(vary);
dsply ('LEN VARY ' + %char(n));
n = strlen(fixed + vary);
dsply ('LEN CAT ' + %char(n));
n = atoi('  42abc');
dsply ('ATOI ' + %char(n));
n = atoi('-7');
dsply ('ATOI NEG ' + %char(n));
if strcmp('abc' : %trim(fixed)) = 0;
  dsply ('CMP EQ');
endif;
if strcmp('abd' : 'abc') > 0;
  dsply ('CMP GT');
endif;
if strncmp('abcX' : 'abcY' : 3) = 0;
  dsply ('NCMP EQ');
endif;
n = strlen(x'C1C2' + 'C');
dsply ('LEN HEX ' + %char(n));
*inlr = *on;
