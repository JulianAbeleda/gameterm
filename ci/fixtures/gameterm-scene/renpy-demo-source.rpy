# GameTerm-owned Ren'Py-shaped fixture source.
# This is not copied from the Ren'Py demo. It exists so CI can verify the
# importer without vendoring third-party script text or assets.

default met_kiki = True

label start:
    scene school_classroom
    show kiki neutral at center
    "A terminal window glows like a tiny stage."
    kiki "Scene Mode can read a Ren'Py-shaped script."

    menu:
        "Ask about Scene Mode." if met_kiki:
            jump explain
        "End the demo.":
            jump ending

label explain:
    show kiki happy at right
    kiki "Labels become dialogue targets, and menu items become Scene Mode choices."
    jump ending

label ending:
    hide kiki
    "The imported demo is ready."
    return
