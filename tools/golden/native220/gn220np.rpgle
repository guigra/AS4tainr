**FREE
// Phase 2: *NULL plus a number: the pointer it gives (compared with
// *NULL, and less *NULL).
ctl-opt dftactgrp(*no);
dcl-s p pointer;
dcl-s q pointer;
dcl-s n int(20);
p = *null;
monitor;
  p = p + 1;
  say('PLUS OK');
on-error;
  say('PLUS ERROR ' + %char(%status));
endmon;
if p = *null;
  say('IS NULL');
else;
  say('NOT NULL');
endif;
q = *null;
monitor;
  n = p - q;
  say('DIFF ' + %char(n));
on-error;
  say('DIFF ERROR ' + %char(%status));
endmon;
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
