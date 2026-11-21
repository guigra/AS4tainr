**FREE
// Native unit NPTRCAL (S2.204): a C call returning a pointer compared
// with *NULL, the program captured on PUB400 as GN204LT with DSPLY for its
// output.
// Source: golden capture tools/golden/native204 n204_ptrcall (PUB400,
// IBM i 7.5, 2026-11-21).
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
dsply (line);
*inlr = *on;
