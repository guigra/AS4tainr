**FREE
// %EDITW with a floating currency symbol: which positions take digits
// when the $ stands just before the 0. An 8,2 field and the same word; a 9,2 field with '   ,   , $0.  '. Captured on PUB400
// (tools/golden/native39.txt) for the interpreter's edit_word.
ctl-opt dftactgrp(*no);
dcl-s a packed(8:2) inz(1000.00);
dcl-s b packed(8:2) inz(.05);
dcl-s c packed(8:2) inz(123456.78);
dcl-s d packed(9:2) inz(.05);
dcl-s e packed(9:2) inz(1234567.89);
say('EF8 1000 [' + %editw(a:'  ,  $0.  ') + ']');
say('EF8 005 [' + %editw(b:'  ,  $0.  ') + ']');
say('EF8 123456 [' + %editw(c:'  ,  $0.  ') + ']');
say('EF9 005 [' + %editw(d:'   ,   , $0.  ') + ']');
say('EF9 1234567 [' + %editw(e:'   ,   , $0.  ') + ']');
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
