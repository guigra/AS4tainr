**FREE
// Native unit NNEST2 (S2.77): a path with two indexes (gv(s).m(i).parmNo),
// literal and field indexes, an index that is such a path itself, an
// inner index past its elements caught by MONITOR. The program captured
// on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native77 n77_nested2 (PUB400, IBM i
// 7.5, 2026-11-18).
dcl-ds m_t qualified template;
  parmNo int(10);
  val char(8);
end-ds;
dcl-ds gv qualified dim(3);
  nm int(10);
  m likeds(m_t) dim(4);
end-ds;
dcl-s s int(10) inz(2);
dcl-s i int(10);
dcl-s line varchar(52);
gv(s).nm = 3;
for i = 1 to 3;
  gv(s).m(i).parmNo = i * 10;
  gv(s).m(i).val = 'v' + %char(i);
endfor;
gv(3).m(4).parmNo = gv(s).m(gv(s).nm).parmNo + gv(2).m(1).parmNo;
line = 'GV';
for i = 1 to gv(s).nm;
  line += ' ' + %char(gv(s).m(i).parmNo) + %trim(gv(s).m(i).val);
endfor;
dsply (line + ' L ' + %char(gv(3).m(4).parmNo) + ' ' + %char(gv(1).m(4).parmNo));
i = 5;
monitor;
  gv(s).m(i).parmNo = 1;
  dsply ('NO ERROR');
on-error;
  dsply ('ERROR ' + %char(%status));
endmon;
*inlr = *on;
