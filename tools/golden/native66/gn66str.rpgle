**FREE
// Stage 2, S2.66: OPTIONS(*STRING) for C functions. A literal, a fixed
// character field (its trailing blanks included), %TRIM of it, a varying
// field, a concatenation and a hexadecimal literal as null-terminated
// strings for strlen, atoi, strcmp and strncmp. Captured on PUB400
// (tools/golden/native66.txt n66_str) for the native unit NSTRING.
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
say('LEN LIT ' + %char(n));
n = strlen(fixed);
say('LEN FIXED ' + %char(n));
n = strlen(%trim(fixed));
say('LEN TRIM ' + %char(n));
n = strlen(vary);
say('LEN VARY ' + %char(n));
n = strlen(fixed + vary);
say('LEN CAT ' + %char(n));
n = atoi('  42abc');
say('ATOI ' + %char(n));
n = atoi('-7');
say('ATOI NEG ' + %char(n));
if strcmp('abc' : %trim(fixed)) = 0;
  say('CMP EQ');
endif;
if strcmp('abd' : 'abc') > 0;
  say('CMP GT');
endif;
if strncmp('abcX' : 'abcY' : 3) = 0;
  say('NCMP EQ');
endif;
n = strlen(x'C1C2' + 'C');
say('LEN HEX ' + %char(n));
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
