# GameTerm-owned Ren'Py-shaped fixture source.
# This is not copied from the Ren'Py demo. It exists so CI can verify the
# importer without vendoring third-party script text or assets.

default met_guide = True

label start:
    "A terminal window glows like a tiny stage."
    guide "Scene Mode can read a Ren'Py-shaped script."

    menu:
        "Ask about Scene Mode." if met_guide:
            jump explain
        "End the demo.":
            jump ending

label explain:
    guide "Labels become dialogue targets, and menu items become Scene Mode choices."
    jump ending

label ending:
    "The imported demo is ready."
    return
