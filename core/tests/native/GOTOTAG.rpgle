      * GOTO, TAG and CABxx: a loop as a state machine. CABLT branches
      * to LOOP while N is less than 5 (IBM Docs, CABxx), so N ends at 5.
     C                   Z-ADD     0             N                 5 0
     C     LOOP          TAG
     C                   ADD       1             N
     C     N             CABLT     5             LOOP
     C     N             CABEQ     5             DONE
     C     'GOTO FAIL'   DSPLY
     C                   GOTO      END
     C     DONE          TAG
     C     'GOTO OK'     DSPLY
     C     END           TAG
     C                   SETON                                        LR
