**FREE
// Stage 2, S2.209: a data structure array subfield of an element of a
// data structure array (gv(s).m) passed to a CONST LIKEDS DIM parameter
// (as rla_test.rs IMQPATH does). Captured on PUB400
// (tools/golden/native209.txt n209_dspath) for the native unit NDSPATH.
ctl-opt dftactgrp(*no);
dcl-ds m_t qualified template;
  parmNo int(10);
  val varchar(20);
end-ds;
dcl-ds gv qualified dim(3) inz;
  nm int(10);
  m likeds(m_t) dim(8);
end-ds;
dcl-s s int(10) inz(3);
dcl-s line varchar(200);

gv(s).nm = 3;
gv(s).m(1).parmNo = 1;
gv(s).m(2).parmNo = 2;
gv(s).m(3).parmNo = 3;
gv(s).m(3).val = 'three';
gv(1).m(1).parmNo = 9;
line = show(gv(s).m : gv(s).nm) + '|' + show(gv(1).m : 1);
say(line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(50);
    m likeds(m_t) dim(8) const;
    n int(10) const;
  end-pi;
  dcl-s i int(10);
  dcl-s o varchar(50);
  for i = 1 to n;
    o += %char(m(i).parmNo) + ' ';
  endfor;
  return o + m(3).val;
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
