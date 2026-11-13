**FREE
// %SUBST with constant arguments out of range, for the review of
// upstream PR #12: a start of 0, a named constant start, and a varchar
// field with a start past its declared length.
ctl-opt dftactgrp(*no);
dcl-c START 11;
dcl-s a char(10) inz('ABCDEFGHIJ');
dcl-s v varchar(10) inz('ABC');
dcl-s c char(5);
c = %subst(a:0:1);
c = %subst(a:START);
c = %subst(v:11);
c = %subst(v:10:1);
c = %subst(a:2:0);
*inlr = *on;
