**FREE
// Stage 2, S2.204: a C function returning a pointer compared with *NULL
// (localtime_r filling a CONST LIKEDS structure, as rla_test.rs LOCALTR
// does), then strftime of it. Captured on PUB400 (tools/golden/native204.txt
// n204_ptrcall) for the native unit NPTRCAL.
ctl-opt dftactgrp(*no);
dcl-ds tm_t qualified template;
  sec int(10);
  min int(10);
  hour int(10);
  mday int(10);
  mon int(10);
  year int(10);
  wday int(10);
  yday int(10);
  isdst int(10);
end-ds;
dcl-pr localtime_r pointer extproc('localtime_r');
  time int(10) const;
  tm likeds(tm_t) const;
end-pr;
dcl-pr strftime int(10) extproc('strftime');
  dest pointer value;
  maxsize int(10) value;
  format pointer value options(*string);
  timeptr const likeds(tm_t);
end-pr;
dcl-ds t likeds(tm_t) inz;
dcl-s secs int(10) inz(864000);
dcl-s out char(11) inz(*allx'00');
dcl-s line varchar(200);

if localtime_r(secs : t) <> *null;
  line = %char(t.year) + ' ' + %char(t.mon) + ' ' + %char(t.mday);
  if strftime(%addr(out) : %size(out) : '%Y-%m-%d' : t) = 10;
    line += ' ' + %subst(out : 1 : 10);
  endif;
endif;
say(line);
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
