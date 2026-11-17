**FREE
// Stage 2, S2.56: C runtime functions and a CEE API called through
// prototypes: strlen, toupper, isdigit, memset, memcpy, strcmp, atoi, and
// CEEDYWK with its feedback code. Captured on PUB400
// (tools/golden/native57.txt n57_c) for the native unit NCFUNC.
ctl-opt dftactgrp(*no) bnddir('QC2LE');
dcl-pr strlen int(10) extproc('strlen');
  s pointer value;
end-pr;
dcl-pr toupper int(10) extproc('toupper');
  c int(10) value;
end-pr;
dcl-pr isdigit int(10) extproc('isdigit');
  c int(10) value;
end-pr;
dcl-pr memset pointer extproc('memset');
  p pointer value;
  c int(10) value;
  n uns(10) value;
end-pr;
dcl-pr memcpy pointer extproc('memcpy');
  d pointer value;
  s pointer value;
  n uns(10) value;
end-pr;
dcl-pr strcmp int(10) extproc('strcmp');
  a pointer value;
  b pointer value;
end-pr;
dcl-pr atoi int(10) extproc('atoi');
  s pointer value;
end-pr;
dcl-pr ceedywk extproc('CEEDYWK');
  lilian int(10) const;
  wkday int(10);
  fc char(12);
end-pr;
dcl-s buf char(12) inz('hello');
dcl-s buf2 char(12);
dcl-s num char(6);
dcl-s n int(10);
dcl-s wd int(10);
dcl-s fc char(12);
%subst(buf : 6 : 1) = x'00';
n = strlen(%addr(buf));
say('STRLEN ' + %char(n));
n = toupper(97);
say('TOUPPER ' + %char(n));
if isdigit(55) <> 0;
  say('ISDIGIT 7 YES');
endif;
if isdigit(65) = 0;
  say('ISDIGIT A NO');
endif;
memset(%addr(buf2) : 64 : %size(buf2));
memcpy(%addr(buf2) : %addr(buf) : 6);
say('MEMCPY [' + %subst(buf2 : 1 : 5) + ']');
n = strcmp(%addr(buf) : %addr(buf2));
say('STRCMP ' + %char(n));
num = '123' + x'00';
n = atoi(%addr(num));
say('ATOI ' + %char(n));
ceedywk(162201 : wd : fc);
say('CEEDYWK ' + %char(wd));
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
