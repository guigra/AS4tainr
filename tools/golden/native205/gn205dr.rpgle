**FREE
// Stage 2, S2.205: a procedure returning a data structure array
// (LIKEDS DIM(2)) assigned to one: RETURN of the array and EVAL of the
// caller's array to the call. Captured on PUB400
// (tools/golden/native205.txt n205_dsret) for the native unit NDSRET.
ctl-opt dftactgrp(*no);
dcl-ds ent_t qualified template;
  pgm char(4);
  n int(10);
end-ds;
dcl-pr mk likeds(ent_t) dim(2) end-pr;
dcl-ds e likeds(ent_t) dim(2);
dcl-s line varchar(200);

e(2).pgm = 'old';
e = mk();
line = e(1).pgm + %char(e(1).n) + '|' + e(2).pgm + %char(e(2).n);
say(line);
*inlr = *on;

dcl-proc mk;
  dcl-pi *n likeds(ent_t) dim(2) end-pi;
  dcl-ds res likeds(ent_t) dim(2);
  res(1).pgm = 'ONE';
  res(1).n = 1;
  res(2).pgm = 'TWO';
  res(2).n = 2;
  return res;
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
