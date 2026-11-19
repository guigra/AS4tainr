**FREE
// Stage 2, S2.130: a field based on a pointer that is itself based (a
// subfield of a based data structure), in a procedure and globally:
// read, changed through %SUBST and returned. Captured on PUB400
// (tools/golden/native130.txt n130_based2) for the native unit NBASED2.
ctl-opt dftactgrp(*no);
dcl-ds h_t qualified template;
  p pointer;
end-ds;
dcl-s buf char(20) inz('hello');
dcl-ds hh likeds(h_t);
dcl-s gp pointer;
dcl-ds g likeds(h_t) based(gp);
dcl-s gs char(3) based(g.p);
dcl-s line varchar(80);

hh.p = %addr(buf);
line = info(%addr(hh));
line += ' ' + %subst(buf: 1: 5);
gp = %addr(hh);
line += ' ' + gs;
gs = 'XYZ';
line += ' ' + %subst(buf: 1: 5);
say(line);
*inlr = *on;

dcl-proc info;
  dcl-pi *n char(5);
    hp pointer const;
  end-pi;
  dcl-ds h likeds(h_t) based(hp);
  dcl-s s char(5) based(h.p);
  %subst(s: 1: 1) = 'J';
  return s;
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
